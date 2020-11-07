use async_std::io;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

pub struct CancelationToken {
	shared_state: Arc<Mutex<CancelationTokenState>>
}

struct CancelationTokenState {
	canceled: bool,
	waker: Option<Waker>,
	task: Option<Box<dyn Future<Output = io::Result<()>> + Send + Sync + Unpin +'static>>
}

impl CancelationToken {
	pub fn new() -> CancelationToken {
		CancelationToken {
			shared_state: Arc::new(Mutex::new(CancelationTokenState {
				canceled: false,
				waker: None,
				task: None
			}))
		}
	}

	pub(crate) fn set_task(&self, task: Box<dyn Future<Output = io::Result<()>> + Send + Sync + Unpin + 'static>) {
		let mut shared_state = self.shared_state.lock().unwrap();

		shared_state.task = Some(task);
	}

	pub async fn complete(&self) -> io::Result<()> {
		let task = {
			let mut shared_state = self.shared_state.lock().unwrap();

			shared_state.canceled = true;
			if let Some(waker) = shared_state.waker.take() {
				waker.wake();
			}

			shared_state.task.take()
		};

		match task {
			Some(task) => task.await,
			None => Ok(())
		}
	}
}

impl Future for CancelationToken {
	type Output = ();

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		let mut shared_state = self.shared_state.lock().unwrap();

		if shared_state.canceled {
            Poll::Ready(())
		} else {
            shared_state.waker = Some(cx.waker().clone());
            Poll::Pending
		}
	}
}

impl Clone for CancelationToken {
	fn clone(&self) -> Self {
		CancelationToken {
			shared_state: self.shared_state.clone()
		}
	}
}

impl fmt::Debug for CancelationToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut shared_state = self.shared_state.lock().unwrap();

		f.debug_struct("CancelationToken")
         .field("canceled", &shared_state.canceled)
         .finish()
    }
}