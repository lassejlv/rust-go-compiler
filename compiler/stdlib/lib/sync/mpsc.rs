//! Multi-producer, single-consumer FIFO queue communication primitives.

use super::sys::{RawCondvar, RawMutex};
use crate::cell::UnsafeCell;
use crate::collections::VecDeque;
use crate::fmt;
use crate::sync::Arc;
use crate::time::{Duration, Instant};

struct State<T> {
    queue: VecDeque<T>,
    senders: usize,
    receiver_alive: bool,
    // bound of a sync channel (None: unbounded)
    cap: Option<usize>,
    // number of messages received so far (for rendezvous channels)
    received: u64,
    // number of messages sent so far
    sent: u64,
}

struct Shared<T> {
    lock: RawMutex,
    not_empty: RawCondvar,
    not_full: RawCondvar,
    state: UnsafeCell<State<T>>,
}

impl<T> Shared<T> {
    #[inline]
    fn locked(&self) -> &mut State<T> {
        unsafe { &mut *self.state.get() }
    }
}

fn new_shared<T>(cap: Option<usize>) -> Arc<Shared<T>> {
    Arc::new(Shared {
        lock: RawMutex::new(),
        not_empty: RawCondvar::new(),
        not_full: RawCondvar::new(),
        state: UnsafeCell::new(State { queue: VecDeque::new(), senders: 1, receiver_alive: true, cap, received: 0, sent: 0 }),
    })
}

fn duration_ns(d: Duration) -> u64 {
    let n = d.as_nanos();
    if n >= (u64::MAX as u128) {
        u64::MAX - 1
    } else {
        n as u64
    }
}

/// Creates a new asynchronous channel, returning the sender/receiver halves.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let s = new_shared(None);
    (Sender { shared: s.clone() }, Receiver { shared: s })
}

/// Creates a new synchronous, bounded channel.
pub fn sync_channel<T>(bound: usize) -> (SyncSender<T>, Receiver<T>) {
    let s = new_shared(Some(bound));
    (SyncSender { shared: s.clone() }, Receiver { shared: s })
}

/// The sending-half of Rust's asynchronous `channel` type.
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

/// The sending-half of Rust's synchronous `sync_channel` type.
pub struct SyncSender<T> {
    shared: Arc<Shared<T>>,
}

/// The receiving half of Rust's `channel` (or `sync_channel`) type.
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

/// An error returned from the `Sender::send` or `SyncSender::send` function on channels.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct SendError<T>(pub T);

/// An error returned from the `recv` function on a `Receiver`.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct RecvError;

/// This enumeration is the list of the possible reasons that `try_recv` could not return data when called.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum TryRecvError {
    /// This channel is currently empty, but the `Sender`(s) have not yet disconnected.
    Empty,
    /// The channel's sending half has become disconnected.
    Disconnected,
}

/// This enumeration is the list of possible errors that made `recv_timeout` unable to return data when called.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum RecvTimeoutError {
    /// This channel is currently empty, but the `Sender`(s) have not yet disconnected.
    Timeout,
    /// The channel's sending half has become disconnected.
    Disconnected,
}

/// This enumeration is the list of the possible error outcomes for the `try_send` method.
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum TrySendError<T> {
    /// The data could not be sent on the `sync_channel` because it would require that the callee block.
    Full(T),
    /// This `sync_channel`'s receiving half has disconnected.
    Disconnected(T),
}

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendError").finish_non_exhaustive()
    }
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("sending on a closed channel")
    }
}

impl<T> crate::error::Error for SendError<T> {}

impl<T> fmt::Debug for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TrySendError::Full(..) => f.write_str("TrySendError::Full(..)"),
            TrySendError::Disconnected(..) => f.write_str("TrySendError::Disconnected(..)"),
        }
    }
}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TrySendError::Full(..) => f.write_str("sending on a full channel"),
            TrySendError::Disconnected(..) => f.write_str("sending on a closed channel"),
        }
    }
}

impl<T> crate::error::Error for TrySendError<T> {}

impl<T> From<SendError<T>> for TrySendError<T> {
    fn from(err: SendError<T>) -> TrySendError<T> {
        TrySendError::Disconnected(err.0)
    }
}

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("receiving on a closed channel")
    }
}

impl crate::error::Error for RecvError {}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TryRecvError::Empty => f.write_str("receiving on an empty channel"),
            TryRecvError::Disconnected => f.write_str("receiving on a closed channel"),
        }
    }
}

impl crate::error::Error for TryRecvError {}

impl From<RecvError> for TryRecvError {
    fn from(_err: RecvError) -> TryRecvError {
        TryRecvError::Disconnected
    }
}

impl fmt::Display for RecvTimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            RecvTimeoutError::Timeout => f.write_str("timed out waiting on channel"),
            RecvTimeoutError::Disconnected => f.write_str("channel is empty and sending half is closed"),
        }
    }
}

impl crate::error::Error for RecvTimeoutError {}

impl From<RecvError> for RecvTimeoutError {
    fn from(_err: RecvError) -> RecvTimeoutError {
        RecvTimeoutError::Disconnected
    }
}

// ---------------------------------------------------------------------------
// sending

// Sends `t`, blocking while a bounded channel is full (and, for a rendezvous
// channel, until the message has been received).
fn send_impl<T>(shared: &Shared<T>, t: T, block: bool) -> Result<(), TrySendError<T>> {
    shared.lock.lock();
    let st = shared.locked();
    if !st.receiver_alive {
        shared.lock.unlock();
        return Err(TrySendError::Disconnected(t));
    }
    match st.cap {
        None => {
            st.queue.push_back(t);
            st.sent += 1;
            shared.lock.unlock();
            shared.not_empty.notify_one();
            Ok(())
        }
        Some(cap) => {
            let limit = if cap == 0 { 1 } else { cap };
            while st.queue.len() >= limit {
                if !block {
                    shared.lock.unlock();
                    return Err(TrySendError::Full(t));
                }
                shared.not_full.wait(&shared.lock, u64::MAX);
                if !st.receiver_alive {
                    shared.lock.unlock();
                    return Err(TrySendError::Disconnected(t));
                }
            }
            if cap == 0 && !block {
                // A rendezvous try_send only succeeds when a receiver is waiting:
                // approximate it by never succeeding.
                shared.lock.unlock();
                return Err(TrySendError::Full(t));
            }
            st.queue.push_back(t);
            st.sent += 1;
            let ticket = st.sent;
            shared.not_empty.notify_all();
            if cap == 0 {
                while st.received < ticket && st.receiver_alive {
                    shared.not_full.wait(&shared.lock, u64::MAX);
                }
            }
            shared.lock.unlock();
            Ok(())
        }
    }
}

impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could not be sent.
    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        match send_impl(&self.shared, t, true) {
            Ok(()) => Ok(()),
            Err(TrySendError::Disconnected(t)) => Err(SendError(t)),
            Err(TrySendError::Full(t)) => Err(SendError(t)),
        }
    }
}

impl<T> SyncSender<T> {
    /// Sends a value on this synchronous channel, blocking while the channel is full.
    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        match send_impl(&self.shared, t, true) {
            Ok(()) => Ok(()),
            Err(TrySendError::Disconnected(t)) => Err(SendError(t)),
            Err(TrySendError::Full(t)) => Err(SendError(t)),
        }
    }

    /// Attempts to send a value on this channel without blocking.
    pub fn try_send(&self, t: T) -> Result<(), TrySendError<T>> {
        send_impl(&self.shared, t, false)
    }
}

fn add_sender<T>(shared: &Arc<Shared<T>>) -> Arc<Shared<T>> {
    shared.lock.lock();
    shared.locked().senders += 1;
    shared.lock.unlock();
    shared.clone()
}

fn drop_sender<T>(shared: &Shared<T>) {
    shared.lock.lock();
    let st = shared.locked();
    st.senders -= 1;
    let last = st.senders == 0;
    shared.lock.unlock();
    if last {
        shared.not_empty.notify_all();
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender { shared: add_sender(&self.shared) }
    }
}

impl<T> Clone for SyncSender<T> {
    fn clone(&self) -> SyncSender<T> {
        SyncSender { shared: add_sender(&self.shared) }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        drop_sender(&self.shared);
    }
}

impl<T> Drop for SyncSender<T> {
    fn drop(&mut self) {
        drop_sender(&self.shared);
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").finish_non_exhaustive()
    }
}

impl<T> fmt::Debug for SyncSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyncSender").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// receiving

impl<T> Receiver<T> {
    // Pops a message (the lock must be held).
    fn pop_locked(&self) -> Option<T> {
        let st = self.shared.locked();
        match st.queue.pop_front() {
            Some(t) => {
                st.received += 1;
                if st.cap.is_some() {
                    self.shared.not_full.notify_all();
                }
                Some(t)
            }
            None => None,
        }
    }

    /// Attempts to return a pending value on this receiver without blocking.
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.shared.lock.lock();
        let r = match self.pop_locked() {
            Some(t) => Ok(t),
            None => {
                if self.shared.locked().senders == 0 {
                    Err(TryRecvError::Disconnected)
                } else {
                    Err(TryRecvError::Empty)
                }
            }
        };
        self.shared.lock.unlock();
        r
    }

    /// Attempts to wait for a value on this receiver, returning an error if the corresponding channel has hung up.
    pub fn recv(&self) -> Result<T, RecvError> {
        self.shared.lock.lock();
        loop {
            if let Some(t) = self.pop_locked() {
                self.shared.lock.unlock();
                return Ok(t);
            }
            if self.shared.locked().senders == 0 {
                self.shared.lock.unlock();
                return Err(RecvError);
            }
            self.shared.not_empty.wait(&self.shared.lock, u64::MAX);
        }
    }

    /// Attempts to wait for a value on this receiver, returning an error if the corresponding channel has hung up,
    /// or if it waits more than `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let start = Instant::now();
        self.shared.lock.lock();
        loop {
            if let Some(t) = self.pop_locked() {
                self.shared.lock.unlock();
                return Ok(t);
            }
            if self.shared.locked().senders == 0 {
                self.shared.lock.unlock();
                return Err(RecvTimeoutError::Disconnected);
            }
            let left = match timeout.checked_sub(start.elapsed()) {
                Some(d) if !d.is_zero() => d,
                _ => {
                    self.shared.lock.unlock();
                    return Err(RecvTimeoutError::Timeout);
                }
            };
            self.shared.not_empty.wait(&self.shared.lock, duration_ns(left));
        }
    }

    /// Attempts to wait for a value on this receiver until `deadline`.
    pub fn recv_deadline(&self, deadline: Instant) -> Result<T, RecvTimeoutError> {
        let now = Instant::now();
        let timeout = if deadline > now { deadline - now } else { Duration::ZERO };
        self.recv_timeout(timeout)
    }

    /// Returns an iterator that will block waiting for messages, but never panic.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { rx: self }
    }

    /// Returns an iterator that will attempt to yield all pending values for a receiver.
    pub fn try_iter(&self) -> TryIter<'_, T> {
        TryIter { rx: self }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.lock.lock();
        let st = self.shared.locked();
        st.receiver_alive = false;
        // Drop the queued messages now (like the real implementation).
        let pending = crate::mem::take(&mut st.queue);
        self.shared.lock.unlock();
        self.shared.not_full.notify_all();
        drop(pending);
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver").finish_non_exhaustive()
    }
}

/// An iterator over messages on a `Receiver`, created by `iter`.
pub struct Iter<'a, T: 'a> {
    rx: &'a Receiver<T>,
}

/// An iterator that attempts to yield all pending values for a `Receiver`, created by `try_iter`.
pub struct TryIter<'a, T: 'a> {
    rx: &'a Receiver<T>,
}

/// An owning iterator over messages on a `Receiver`, created by `into_iter`.
pub struct IntoIter<T> {
    rx: Receiver<T>,
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.rx.recv().ok()
    }
}

impl<'a, T> Iterator for TryIter<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.rx.try_recv().ok()
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.rx.recv().ok()
    }
}

impl<'a, T> IntoIterator for &'a Receiver<T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T> IntoIterator for Receiver<T> {
    type Item = T;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> IntoIter<T> {
        IntoIter { rx: self }
    }
}

impl<'a, T> fmt::Debug for Iter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Iter").field("rx", &self.rx).finish()
    }
}

impl<'a, T> fmt::Debug for TryIter<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TryIter").field("rx", &self.rx).finish()
    }
}

impl<T> fmt::Debug for IntoIter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IntoIter").field("rx", &self.rx).finish()
    }
}
