use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use futures::{SinkExt, StreamExt};
use tokio::sync::Mutex;

use crate::{routing::RouteRequest, JsRunner};

#[derive(Clone)]
pub struct WorkerPool {
    workers: Vec<Arc<Mutex<Worker>>>,
    next_worker: Arc<AtomicUsize>,
}

impl WorkerPool {
    pub(crate) fn new(n_workers: usize) -> Self {
        let workers = (0..n_workers)
            .map(|_| Arc::new(Mutex::new(Worker::default())))
            .collect();

        Self {
            workers,
            next_worker: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub(crate) fn next_worker(&self) -> Arc<Mutex<Worker>> {
        let worker = &self.workers[self.next_worker.load(Ordering::SeqCst)];
        self.next_worker
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                Some((current + 1) % self.workers.len())
            })
            .unwrap();

        Arc::clone(worker)
    }
}

pub(crate) struct Worker {
    pub(crate) sender: futures::channel::mpsc::Sender<RouteRequest>,
    pub(crate) receiver:
        futures::channel::mpsc::Receiver<axum::response::Response<axum::body::Body>>,
    pub(crate) handle: std::thread::JoinHandle<()>,
}

impl Worker {
    pub(crate) async fn send(&mut self, req: RouteRequest) {
        self.sender.send(req).await.unwrap();
    }
}

impl Default for Worker {
    fn default() -> Self {
        let (sender, mut receiver) = futures::channel::mpsc::channel(42);
        let (mut client_sender, client_receiver) = futures::channel::mpsc::channel(42);

        let handle = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                let js_runner = JsRunner::new(None).await;

                loop {
                    if let Some(req) = receiver.next().await {
                        let resp = js_runner.run_route(&req).await;
                        client_sender.send(resp).await.unwrap();
                    }
                }
            });
        });

        Self {
            sender,
            handle,
            receiver: client_receiver,
        }
    }
}
