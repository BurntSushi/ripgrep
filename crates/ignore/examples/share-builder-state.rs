use std::{
    collections::BTreeSet,
    mem, ops,
    path::PathBuf,
    ptr,
    sync::{
        atomic::{AtomicPtr, Ordering},
        mpsc, Mutex,
    },
    thread,
};

struct VisitorBuilder {
    traversal_error: AtomicPtr<ignore::Error>,
    all_matches: Mutex<BTreeSet<PathBuf>>,
    result: mpsc::SyncSender<Result<BTreeSet<PathBuf>, ignore::Error>>,
}

impl VisitorBuilder {
    fn new(
        result: mpsc::SyncSender<Result<BTreeSet<PathBuf>, ignore::Error>>,
    ) -> Self {
        Self {
            traversal_error: AtomicPtr::new(ptr::null_mut()),
            all_matches: Mutex::new(BTreeSet::new()),
            result,
        }
    }
}

impl ops::Drop for VisitorBuilder {
    fn drop(&mut self) {
        let err_ptr = self.traversal_error.get_mut();
        if !err_ptr.is_null() {
            let e: Box<ignore::Error> = unsafe { Box::from_raw(*err_ptr) };
            self.result.send(Err(*e)).unwrap();
            return;
        }
        match self
            .result
            .send(Ok(mem::take(&mut self.all_matches.get_mut().unwrap())))
        {
            Ok(()) => (),
            Err(_e) => {
                eprintln!("failed to send result (hangup)");
            }
        }
    }
}

impl ignore::ParallelVisitorBuilder for VisitorBuilder {
    type Visitor<'s>
        = Visitor<'s>
    where
        Self: 's;
    fn build<'s, 't: 's>(&'t self) -> Self::Visitor<'s> {
        Visitor::new(&self.traversal_error, &self.all_matches)
    }
}

struct Visitor<'s> {
    traversal_error: &'s AtomicPtr<ignore::Error>,
    cur_matches: Vec<PathBuf>,
    all_matches: &'s Mutex<BTreeSet<PathBuf>>,
}

impl<'s> Visitor<'s> {
    fn new(
        traversal_error: &'s AtomicPtr<ignore::Error>,
        all_matches: &'s Mutex<BTreeSet<PathBuf>>,
    ) -> Self {
        Self { traversal_error, cur_matches: Vec::new(), all_matches }
    }

    #[inline(always)]
    fn fatal_error_was_signaled(&self) -> bool {
        !self.traversal_error.load(Ordering::Acquire).is_null()
    }

    fn handle_fatal_error(&self, e: ignore::Error) {
        let boxed = Box::into_raw(Box::new(e));
        match self.traversal_error.compare_exchange(
            ptr::null_mut(),
            boxed,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(prev) => {
                debug_assert!(prev.is_null());
            }
            Err(_prior_error) => {
                let e = unsafe { Box::from_raw(boxed) };
                eprintln!("dropped racing error: {}", e);
            }
        }
    }
}

impl<'s> ops::Drop for Visitor<'s> {
    fn drop(&mut self) {
        if !self.traversal_error.load(Ordering::Relaxed).is_null() {
            return;
        }
        self.all_matches.lock().unwrap().extend(self.cur_matches.drain(..));
    }
}

impl<'s> ignore::ParallelVisitor for Visitor<'s> {
    fn visit(
        &mut self,
        entry: Result<ignore::DirEntry, ignore::Error>,
    ) -> ignore::WalkState {
        if self.fatal_error_was_signaled() {
            return ignore::WalkState::Quit;
        }
        match entry {
            Err(e) => {
                self.handle_fatal_error(e);
                ignore::WalkState::Quit
            }
            Ok(entry) => {
                if let Some(e) = entry.error() {
                    eprintln!(
                        "non-fatal error while processing entry {:?}: {}",
                        &entry, e
                    );
                }
                let file_type = entry.file_type().unwrap();
                if file_type.is_file() {
                    self.cur_matches.push(entry.into_path());
                }
                ignore::WalkState::Continue
            }
        }
    }
}

fn main() {
    let (send, recv) = mpsc::sync_channel(0);
    let t = thread::spawn(move || {
        ignore::WalkBuilder::new(".")
            .build_parallel()
            .visit(VisitorBuilder::new(send.clone()));
        ignore::WalkBuilder::new("asdf")
            .build_parallel()
            .visit(VisitorBuilder::new(send));
    });
    for result in recv.iter() {
        println!("result: {:?}", result.unwrap());
    }
    t.join().unwrap();
}
