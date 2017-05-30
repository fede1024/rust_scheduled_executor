use futures::future::Future;
use futures::sync::oneshot::{channel, Sender};
use tokio_core::reactor::{Core, Handle, Remote};
use tokio_core::reactor::Timeout;

use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Instant, Duration};
use std::io;


fn fixed_interval_loop<F>(scheduled_fn: Arc<F>, interval: Duration, handle: &Handle)
    where F: Fn(&Handle) + Send + 'static
{
    let start_time = Instant::now();
    scheduled_fn(&handle);
    let execution = start_time.elapsed();
    let next_iter_wait = if execution >= interval {
        Duration::from_secs(0)
    } else {
        interval - execution
    };
    let handle_clone = handle.clone();
    let scheduled_fn_clone = scheduled_fn.clone();
    let t = Timeout::new(next_iter_wait, handle).unwrap()
        .then(move |_| {
            fixed_interval_loop(scheduled_fn_clone, interval, &handle_clone);
            Ok::<(), ()>(())
        });
    handle.spawn(t);
}

fn calculate_delay(interval: Duration, execution: Duration, delay: Duration) -> (Duration, Duration) {
    if execution >= interval {
        (Duration::from_secs(0), delay + execution - interval)
    } else {
        let wait_gap = interval - execution;
        if delay == Duration::from_secs(0) {
            (wait_gap, Duration::from_secs(0))
        } else {
            if delay < wait_gap {
                (wait_gap - delay, Duration::from_secs(0))
            } else {
                (Duration::from_secs(0), delay - wait_gap)
            }
        }
    }
}

fn fixed_rate_loop<F>(scheduled_fn: Arc<F>, interval: Duration, handle: &Handle, delay: Duration)
    where F: Fn(&Handle) + Send + 'static
{
    let start_time = Instant::now();
    scheduled_fn(&handle);
    let execution = start_time.elapsed();
    let (next_iter_wait, updated_delay) = calculate_delay(interval, execution, delay);
    let handle_clone = handle.clone();
    let scheduled_fn_clone = scheduled_fn.clone();
    let t = Timeout::new(next_iter_wait, handle).unwrap()
        .then(move |_| {
            fixed_rate_loop(scheduled_fn_clone, interval, &handle_clone, updated_delay);
            Ok::<(), ()>(())
        });
    handle.spawn(t);
}

pub struct Executor {
    remote: Remote,
    termination_sender: Sender<()>,
    thread_handle: JoinHandle<()>,
}

impl Executor {
    pub fn new() -> Result<Executor, io::Error> {
        Executor::with_name("executor")
    }

    pub fn with_name(thread_name: &str) -> Result<Executor, io::Error> {
        let (termination_tx, termination_rx) = channel();
        let (core_tx, core_rx) = channel();
        let thread_handle = thread::Builder::new()
            .name(thread_name.to_owned())
            .spawn(move || {
                debug!("Core starting");
                let mut core = Core::new().expect("Failed to start core");
                let _ = core_tx.send(core.remote());
                match core.run(termination_rx) {
                    Ok(v) => debug!("Core terminated correctly {:?}", v),
                    Err(e) => debug!("Core terminated with error: {:?}", e),
                }
            })?;
        let executor = Executor {
            remote: core_rx.wait().expect("Failed to receive remote"),
            termination_sender: termination_tx,
            thread_handle: thread_handle,
        };
        debug!("Executor created");
        Ok(executor)
    }

    pub fn stop_async(self) {
        let _ = self.termination_sender.send(());
    }

    pub fn stop_sync(self) {
        let _ = self.termination_sender.send(());
        let _ = self.thread_handle.join();
    }

    pub fn schedule_fixed_interval<F>(&self, interval: Duration, scheduled_fn: F)
        where F: Fn(&Handle) + Send + 'static
    {
        self.remote.spawn(move |handle| {
            fixed_interval_loop(Arc::new(scheduled_fn), interval, handle);
            Ok::<(), ()>(())
        });
    }

    pub fn schedule_fixed_rate<F>(&self, interval: Duration, scheduled_fn: F)
        where F: Fn(&Handle) + Send + 'static
    {
        self.remote.spawn(move |handle| {
            fixed_rate_loop(Arc::new(scheduled_fn), interval, handle, Duration::from_secs(0));
            Ok::<(), ()>(())
        });
    }
}


#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{Executor, calculate_delay};

    #[test]
    fn fixed_interval_test() {
        let timings = Arc::new(RwLock::new(Vec::new()));
        let executor = Executor::new().unwrap();
        let timings_clone = Arc::clone(&timings);
        executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
            timings_clone.write().unwrap().push(Instant::now());
        });
        thread::sleep(Duration::from_millis(5500));
        executor.stop_sync();

        let timings = timings.read().unwrap();
        assert!(timings.len() == 6);
        for i in 1..6 {
            let execution_interval = timings[i] - timings[i-1];
            assert!(execution_interval < Duration::from_millis(1020));
            assert!(execution_interval > Duration::from_millis(980));
        }
    }

    #[test]
    fn fixed_interval_slow_task_test() {
        let executor = Executor::new().unwrap();
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);
        executor.schedule_fixed_interval(Duration::from_secs(1), move |_handle| {
            let mut counter = counter_clone.write().unwrap();
            (*counter) += 1;
            if *counter == 1 {
                thread::sleep(Duration::from_secs(3));
            }
        });
        thread::sleep(Duration::from_millis(5500));
        executor.stop_sync();
        assert_eq!(*counter.read().unwrap(), 4);
    }

    #[test]
    fn calculate_delay_test() {
        fn s(n: u64) -> Duration { Duration::from_secs(n) };
        assert_eq!(calculate_delay(s(10), s(3), s(0)), (s(7), s(0)));
        assert_eq!(calculate_delay(s(10), s(11), s(0)), (s(0), s(1)));
        assert_eq!(calculate_delay(s(10), s(3), s(3)), (s(4), s(0)));
        assert_eq!(calculate_delay(s(10), s(3), s(9)), (s(0), s(2)));
        assert_eq!(calculate_delay(s(10), s(12), s(15)), (s(0), s(17)));
    }

    #[test]
    fn fixed_rate_test() {
        let executor = Executor::new().unwrap();
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);
        executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
            let mut counter = counter_clone.write().unwrap();
            (*counter) += 1;
        });
        thread::sleep(Duration::from_millis(5500));
        executor.stop_sync();
        assert_eq!(*counter.read().unwrap(), 6);
    }

    #[test]
    fn fixed_rate_slow_task_test() {
        let executor = Executor::new().unwrap();
        let counter = Arc::new(RwLock::new(0));
        let counter_clone = Arc::clone(&counter);
        executor.schedule_fixed_rate(Duration::from_secs(1), move |_handle| {
            let mut counter = counter_clone.write().unwrap();
            (*counter) += 1;
            if *counter == 1 {
                thread::sleep(Duration::from_secs(3));
            }
        });
        thread::sleep(Duration::from_millis(5500));
        executor.stop_sync();
        assert_eq!(*counter.read().unwrap(), 6);
    }
}
