use std::{
    collections::VecDeque,
    io::{Read, Write},
    net::TcpStream,
    str::Split,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Condvar, Mutex,
    },
};

use http::Request;

use crate::{
    http_utils::{RequestFromBytes, ResponseToBytes},
    routing::Route,
    systems::RawResponse,
};

const MIN_TASK: usize = 4;

pub struct Task {
    pub stream: TcpStream,
    pub router: Arc<Route>,
}

pub struct Context<'b> {
    pub request: Request<Vec<u8>>,
    pub path_iter: Split<'b, &'static str>,
}

struct Shared {
    /// Pool of tasks that need to be run
    pool: Mutex<VecDeque<Task>>,

    /// Conditional var used to sleep and wake threads
    condvar: Condvar,

    active_tasks: AtomicUsize,

    /// Total number of threads currently waiting for a task
    waiting_tasks: AtomicUsize,
}

impl Shared {
    fn active(&self) {
        self.waiting_tasks.fetch_sub(1, Ordering::Release);
    }

    fn waiting(&self) {
        self.waiting_tasks.fetch_add(1, Ordering::Release);
    }
}

pub struct TaskPool {
    shared: Arc<Shared>,
}

impl TaskPool {
    pub fn new() -> Self {
        let mut pool = TaskPool {
            shared: Arc::new(Shared {
                pool: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                active_tasks: AtomicUsize::new(0),
                waiting_tasks: AtomicUsize::new(0),
            }),
        };

        for _ in 0..MIN_TASK {
            pool.spawn_thread();
        }

        pool
    }

    pub fn spawn_thread(&mut self) {

        let thread_count = self.shared.active_tasks.fetch_add(1, Ordering::Acquire);

        println!("Spawning thread: {}", thread_count);

        let shared = self.shared.clone();

        std::thread::spawn(move || {

            loop {
                let mut pool = shared.pool.lock().unwrap();

                shared.waiting();
                pool = shared.condvar.wait(pool).unwrap();
                shared.active();

                let Some(task) = pool.pop_front() else {
                    continue;
                };

                drop(pool);

                handle_connection(task);
            }
        });
    }

    pub fn send_task(&mut self, task: Task) {
        self.shared.pool.lock().unwrap().push_back(task);

        if self.shared.waiting_tasks.load(Ordering::Acquire) == 0 {
            self.spawn_thread();
        }

        self.shared.condvar.notify_one();
    }
}

fn handle_connection(mut task: Task) {
    let mut buf: [u8; 1024] = [0; 1024];

    match task.stream.read(&mut buf) {
        Ok(n) => {
            if n == 0 {
                return;
            }

            let request = Request::try_from_bytes(&buf[..n]).expect("Failed to parse request");

            handle_request(task, request.map(|f| f.into_bytes()));
        }
        Err(_) => {},
    }
}

fn handle_request(task: Task, request: Request<Vec<u8>>) {
    let path = request.uri().path().to_string();

    let mut path_iter = path.split("/");

    // Discard first in iter as it will always be an empty string
    path_iter.next();

    let mut ctx = Context { request, path_iter };

    let mut cursor = task.router.as_ref();

    loop {
        for system in cursor.systems() {
            if let Some(r) = system.call(&mut ctx) {
                respond(task.stream, r);
                return;
            }
        }

        let Some(next) = ctx.path_iter.next() else {
            break;
        };

        if let Some(child) = cursor.get_child(next) {
            cursor = child;
        } else {
            break;
        }
    }
}

fn respond(mut stream: TcpStream, response: RawResponse) {
    let _ = stream.write_all(&response.into_bytes());

    let _ = stream.flush();
}
