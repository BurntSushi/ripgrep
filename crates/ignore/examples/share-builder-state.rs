use std::{
    collections::BTreeSet,
    ops,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};

struct VisitorBuilder {
    traversal_error: OnceLock<ignore::Error>,
    all_matches: Mutex<BTreeSet<PathBuf>>,
}

impl VisitorBuilder {
    fn new() -> Self {
        Self {
            traversal_error: OnceLock::new(),
            all_matches: Mutex::new(BTreeSet::new()),
        }
    }

    fn into_result(self) -> Result<BTreeSet<PathBuf>, ignore::Error> {
        let Self { traversal_error, all_matches } = self;
        if let Some(e) = traversal_error.into_inner() {
            Err(e)
        } else {
            Ok(all_matches.into_inner().unwrap())
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
    traversal_error: &'s OnceLock<ignore::Error>,
    cur_matches: Vec<PathBuf>,
    all_matches: &'s Mutex<BTreeSet<PathBuf>>,
}

impl<'s> Visitor<'s> {
    fn new(
        traversal_error: &'s OnceLock<ignore::Error>,
        all_matches: &'s Mutex<BTreeSet<PathBuf>>,
    ) -> Self {
        Self { traversal_error, cur_matches: Vec::new(), all_matches }
    }
}

impl<'s> ops::Drop for Visitor<'s> {
    fn drop(&mut self) {
        if self.traversal_error.get().is_some() {
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
        if self.traversal_error.get().is_some() {
            return ignore::WalkState::Quit;
        }
        match entry {
            Err(e) => {
                let _ = self.traversal_error.set(e);
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

fn walk_dir(
    dir: impl AsRef<Path>,
) -> Result<BTreeSet<PathBuf>, ignore::Error> {
    ignore::WalkBuilder::new(dir)
        .build_parallel()
        .visit(VisitorBuilder::new())
        .into_result()
}

fn main() {
    println!("success: {:?}", walk_dir(".").unwrap());
    println!("err: {:?}", walk_dir("asdf"));
}
