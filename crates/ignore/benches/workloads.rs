//! File discovery and incremental matching workloads used by Python tools.
#![feature(test)]

extern crate test;

use std::{
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use ignore::{
    DirEntry, Error, ParallelVisitor, ParallelVisitorBuilder, WalkBuilder,
    WalkState, gitignore::GitignoreBuilder, overrides::OverrideBuilder,
    types::TypesBuilder,
};

const RULES: &str = "\
.venv/
__pycache__/
*.py[cod]
*.egg-info/
/build/
/dist/
*.log
";

/// A deterministic source tree, created before timing starts.
struct Workspace {
    temp: PathBuf,
    root: PathBuf,
    paths: Vec<PathBuf>,
    python_files: usize,
}

impl Workspace {
    fn new(packages: usize, depth: usize, files: usize, nested: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let temp = std::env::temp_dir().join(format!(
            "ignore-bench-{:x}-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&temp).unwrap();
        let root = temp.join("workspace");
        let mut ws = Workspace { temp, root, paths: vec![], python_files: 0 };
        ws.write(".git/info/exclude", "private/\n");
        ws.write(".gitignore", RULES);
        ws.write(".ignore", ".git/\n*.tmp\n");
        ws.write(".venv/lib/python3.13/site-packages/dependency.py", "");
        ws.write("private/internal.py", "");
        ws.write("build/generated.py", "");
        for package in 0..packages {
            let base = format!("packages/pkg{package}");
            ws.write(&format!("{base}/pyproject.toml"), "[project]\n");
            if nested {
                ws.write(
                    &format!("{base}/.gitignore"),
                    "generated_*.py\n!generated_keep.py\n",
                );
            }
            let mut dir = base;
            for level in 0..depth {
                dir.push_str(&format!("/level{level}"));
                for file in 0..files {
                    let path = format!("{dir}/module{file}.py");
                    ws.write(&path, "");
                    ws.paths.push(PathBuf::from(path));
                    ws.python_files += 1;
                }
                ws.write(&format!("{dir}/__pycache__/module.pyc"), "");
                ws.write(&format!("{dir}/notes.txt"), "");
                if nested {
                    ws.write(&format!("{dir}/generated_drop.py"), "");
                    ws.write(&format!("{dir}/generated_keep.py"), "");
                    ws.python_files += 1;
                }
            }
        }
        ws
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn builder(&self) -> WalkBuilder {
        let mut builder = WalkBuilder::new(&self.root);
        // Keep parent discovery, but do not depend on the user's Git config.
        builder.current_dir(&self.root).hidden(false).git_global(false);
        builder
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.temp).unwrap();
    }
}

fn is_python(entry: &DirEntry) -> bool {
    entry.file_type().is_some_and(|ft| ft.is_file())
        && entry.path().extension().is_some_and(|ext| ext == "py")
}

/// Accumulate per worker, as Ruff does, instead of contending on every file.
struct CountBuilder<'a>(&'a Mutex<usize>);

impl<'s> ParallelVisitorBuilder<'s> for CountBuilder<'s> {
    fn build(&mut self) -> Box<dyn ParallelVisitor + 's> {
        Box::new(CountVisitor { total: self.0, count: 0 })
    }
}

struct CountVisitor<'a> {
    total: &'a Mutex<usize>,
    count: usize,
}

impl ParallelVisitor for CountVisitor<'_> {
    fn visit(&mut self, entry: Result<DirEntry, Error>) -> WalkState {
        let entry = entry.unwrap();
        assert!(entry.error().is_none());
        self.count += usize::from(is_python(&entry));
        WalkState::Continue
    }
}

impl Drop for CountVisitor<'_> {
    fn drop(&mut self) {
        *self.total.lock().unwrap() += self.count;
    }
}

fn walk(builder: &WalkBuilder, threads: usize) -> usize {
    if threads == 0 {
        builder
            .build()
            .map(|entry| {
                let entry = entry.unwrap();
                assert!(entry.error().is_none());
                usize::from(is_python(&entry))
            })
            .sum()
    } else {
        let total = Mutex::new(0);
        let mut counter = CountBuilder(&total);
        builder.clone().threads(threads).build_parallel().visit(&mut counter);
        total.into_inner().unwrap()
    }
}

macro_rules! walk_bench {
    ($name:ident, $packages:expr, $depth:expr, $files:expr, $nested:expr, $threads:expr) => {
        #[bench]
        fn $name(b: &mut test::Bencher) {
            let ws = Workspace::new($packages, $depth, $files, $nested);
            let builder = ws.builder();
            assert_eq!(walk(&builder, $threads), ws.python_files);
            b.iter(|| {
                assert_eq!(
                    walk(black_box(&builder), $threads),
                    ws.python_files
                )
            });
        }
    };
}

walk_bench!(walk_small_serial, 4, 2, 8, false, 0);
walk_bench!(walk_small_parallel_12, 4, 2, 8, false, 12);
walk_bench!(walk_wide_serial, 128, 1, 32, false, 0);
walk_bench!(walk_wide_parallel_1, 128, 1, 32, false, 1);
walk_bench!(walk_wide_parallel_4, 128, 1, 32, false, 4);
walk_bench!(walk_wide_parallel_12, 128, 1, 32, false, 12);
walk_bench!(walk_deep_serial, 16, 12, 16, false, 0);
walk_bench!(walk_deep_parallel_12, 16, 12, 16, false, 12);
walk_bench!(walk_nested_serial, 64, 3, 16, true, 0);
walk_bench!(walk_nested_parallel_12, 64, 3, 16, true, 12);

#[bench]
fn walk_unfiltered_parallel_12(b: &mut test::Bencher) {
    let ws = Workspace::new(128, 1, 32, false);
    let mut builder = ws.builder();
    builder.standard_filters(false);
    let expected = ws.python_files + 3;
    assert_eq!(walk(&builder, 12), expected);
    b.iter(|| assert_eq!(walk(black_box(&builder), 12), expected));
}

#[bench]
fn walk_explicit_files(b: &mut test::Bencher) {
    let ws = Workspace::new(64, 1, 8, false);
    let mut builder = WalkBuilder::from_iter(
        ws.paths.iter().map(|relative| ws.root.join(relative)),
    );
    builder.current_dir(&ws.root).hidden(false).git_global(false).threads(12);
    b.iter(|| assert_eq!(walk(black_box(&builder), 12), ws.python_files));
}

#[bench]
fn uv_ignored_workspace_member(b: &mut test::Bencher) {
    let ws = Workspace::new(1, 1, 0, false);
    let member = ws.root.join("packages/pkg0/level0");
    fs::remove_file(member.join("notes.txt")).unwrap();
    let mut builder = WalkBuilder::new(&member);
    builder.hidden(false).git_global(false).current_dir(&ws.root);
    b.iter(|| {
        // uv stops as soon as it encounters a non-directory entry.
        assert!(!builder.build().any(|entry| !entry.unwrap().path().is_dir()));
    });
}

#[bench]
fn uv_nonempty_workspace_member(b: &mut test::Bencher) {
    let ws = Workspace::new(1, 1, 32, false);
    let member = ws.root.join("packages/pkg0/level0");
    let mut builder = WalkBuilder::new(&member);
    builder.hidden(false).git_global(false).current_dir(&ws.root);
    b.iter(|| {
        assert!(builder.build().any(|entry| !entry.unwrap().path().is_dir()));
    });
}

#[bench]
fn uv_scripts_source_archive(b: &mut test::Bencher) {
    let ws = Workspace::new(64, 3, 16, true);
    fs::remove_dir_all(ws.root.join(".git")).unwrap();
    let mut builder = ws.builder();
    builder.require_git(false);
    // Without .git/info/exclude, private/internal.py is also visible.
    let expected = ws.python_files + 1;
    assert_eq!(walk(&builder, 0), expected);
    b.iter(|| assert_eq!(walk(black_box(&builder), 0), expected));
}

#[bench]
fn incremental_cold(b: &mut test::Bencher) {
    let ws = Workspace::new(16, 4, 4, true);
    let builder = ws.builder();
    b.iter(|| {
        let mut matchers = builder.build_matchers();
        for path in &ws.paths {
            assert!(!matchers[0].matched(black_box(path), false).is_ignore());
        }
    });
}

#[bench]
fn incremental_warm(b: &mut test::Bencher) {
    let ws = Workspace::new(16, 4, 4, true);
    let mut matchers = ws.builder().build_matchers();
    for path in &ws.paths {
        assert!(!matchers[0].matched(path, false).is_ignore());
    }
    b.iter(|| {
        for path in &ws.paths {
            assert!(!matchers[0].matched(black_box(path), false).is_ignore());
        }
    });
}

#[bench]
fn incremental_warm_noncanonical(b: &mut test::Bencher) {
    let ws = Workspace::new(16, 4, 4, true);
    let paths: Vec<_> = ws
        .paths
        .iter()
        .map(|path| {
            Path::new("packages/.")
                .join(path.strip_prefix("packages").unwrap())
        })
        .collect();
    let mut matchers = ws.builder().build_matchers();
    for path in &paths {
        assert!(!matchers[0].matched(path, false).is_ignore());
    }
    b.iter(|| {
        for path in &paths {
            assert!(!matchers[0].matched(black_box(path), false).is_ignore());
        }
    });
}

#[bench]
fn incremental_ignored(b: &mut test::Bencher) {
    let ws = Workspace::new(0, 0, 0, false);
    let mut matchers = ws.builder().build_matchers();
    let path = Path::new(".venv/lib/python3.13/site-packages/dependency.py");
    assert!(matchers[0].matched(path, false).is_ignore());
    b.iter(|| {
        assert!(matchers[0].matched(black_box(path), false).is_ignore())
    });
}

#[bench]
fn build_empty_gitignore(b: &mut test::Bencher) {
    let builder = GitignoreBuilder::new("workspace/packages/example");
    b.iter(|| black_box(builder.build().unwrap()));
}

#[bench]
fn build_single_directory_whitelist(b: &mut test::Bencher) {
    let mut builder = GitignoreBuilder::new("workspace");
    builder.add_line(None, "!/.github/").unwrap();
    b.iter(|| black_box(builder.build().unwrap()));
}

#[bench]
fn build_python_gitignore(b: &mut test::Bencher) {
    let mut builder = GitignoreBuilder::new("workspace");
    for line in RULES.lines() {
        builder.add_line(None, line).unwrap();
    }
    b.iter(|| black_box(builder.build().unwrap()));
}

#[bench]
fn match_python_gitignore(b: &mut test::Bencher) {
    let mut builder = GitignoreBuilder::new("workspace");
    for line in RULES.lines() {
        builder.add_line(None, line).unwrap();
    }
    let matcher = builder.build().unwrap();
    b.iter(|| {
        assert!(
            matcher
                .matched(black_box("workspace/pkg/mod.py"), false)
                .is_none()
        );
        assert!(
            matcher
                .matched(black_box("workspace/pkg/mod.pyc"), false)
                .is_ignore()
        );
        assert!(
            matcher
                .matched(black_box("workspace/pkg/__pycache__"), true)
                .is_ignore()
        );
    });
}

#[bench]
fn incremental_warm_deep(b: &mut test::Bencher) {
    let ws = Workspace::new(16, 12, 4, true);
    let mut matchers = ws.builder().build_matchers();
    for path in &ws.paths {
        assert!(!matchers[0].matched(path, false).is_ignore());
    }
    b.iter(|| {
        for path in &ws.paths {
            assert!(!matchers[0].matched(black_box(path), false).is_ignore());
        }
    });
}

#[bench]
fn incremental_ignored_nested(b: &mut test::Bencher) {
    let ws = Workspace::new(1, 1, 1, false);
    let path = Path::new(
        "packages/pkg0/level0/.venv/lib/python3.13/site-packages/dependency.py",
    );
    ws.write(path.to_str().unwrap(), "");
    let mut matchers = ws.builder().build_matchers();
    assert!(matchers[0].matched(path, false).is_ignore());
    b.iter(|| {
        assert!(matchers[0].matched(black_box(path), false).is_ignore())
    });
}

#[bench]
fn incremental_ignored_parent(b: &mut test::Bencher) {
    let ws = Workspace::new(0, 0, 0, false);
    let mut matchers = ws.builder().build_matchers();
    let path = Path::new(".venv/dependency.py");
    assert!(matchers[0].matched(path, false).is_ignore());
    b.iter(|| {
        assert!(matchers[0].matched(black_box(path), false).is_ignore())
    });
}

/// Create a deep tree with ignore rules at every package level.
fn dense_ignore_workspace() -> Workspace {
    let ws = Workspace::new(16, 12, 16, true);
    for package in 0..16 {
        let mut dir = format!("packages/pkg{package}");
        for level in 0..12 {
            dir.push_str(&format!("/level{level}"));
            ws.write(
                &format!("{dir}/.gitignore"),
                "generated_*.py\n!generated_keep.py\n",
            );
        }
    }
    ws
}

fn dense_ignore_walk(b: &mut test::Bencher, threads: usize) {
    let ws = dense_ignore_workspace();
    let builder = ws.builder();
    assert_eq!(walk(&builder, threads), ws.python_files);
    b.iter(|| assert_eq!(walk(black_box(&builder), threads), ws.python_files));
}

#[bench]
fn walk_dense_ignore_serial(b: &mut test::Bencher) {
    dense_ignore_walk(b, 0);
}

#[bench]
fn walk_dense_ignore_parallel_12(b: &mut test::Bencher) {
    dense_ignore_walk(b, 12);
}

#[bench]
fn incremental_dense_ignore_cold(b: &mut test::Bencher) {
    let ws = dense_ignore_workspace();
    let builder = ws.builder();
    b.iter(|| {
        let mut matchers = builder.build_matchers();
        for path in &ws.paths {
            assert!(!matchers[0].matched(black_box(path), false).is_ignore());
        }
    });
}

#[bench]
fn incremental_dense_ignore_warm(b: &mut test::Bencher) {
    let ws = dense_ignore_workspace();
    let mut matchers = ws.builder().build_matchers();
    for path in &ws.paths {
        assert!(!matchers[0].matched(path, false).is_ignore());
    }
    b.iter(|| {
        for path in &ws.paths {
            assert!(!matchers[0].matched(black_box(path), false).is_ignore());
        }
    });
}

fn multiple_roots_walk(b: &mut test::Bencher, threads: usize) {
    let ws = Workspace::new(64, 1, 32, false);
    let mut builder = WalkBuilder::new(ws.root.join("packages/pkg0"));
    builder.current_dir(&ws.root).hidden(false).git_global(false);
    for package in 1..64 {
        builder.add(ws.root.join(format!("packages/pkg{package}")));
    }
    assert_eq!(walk(&builder, threads), ws.python_files);
    b.iter(|| assert_eq!(walk(black_box(&builder), threads), ws.python_files));
}

#[bench]
fn walk_multiple_roots_parallel_1(b: &mut test::Bencher) {
    multiple_roots_walk(b, 1);
}

#[bench]
fn walk_multiple_roots_parallel_12(b: &mut test::Bencher) {
    multiple_roots_walk(b, 12);
}

#[bench]
fn walk_with_overrides_and_types(b: &mut test::Bencher) {
    let ws = Workspace::new(64, 3, 16, true);
    let mut overrides = OverrideBuilder::new(&ws.root);
    overrides.add("!*.txt").unwrap();
    let mut types = TypesBuilder::new();
    types.add("python", "*.py").unwrap();
    types.select("python");
    let mut builder = ws.builder();
    builder
        .overrides(overrides.build().unwrap())
        .types(types.build().unwrap());
    assert_eq!(walk(&builder, 12), ws.python_files);
    b.iter(|| assert_eq!(walk(black_box(&builder), 12), ws.python_files));
}

#[bench]
fn match_single_file_glob(b: &mut test::Bencher) {
    let mut builder = GitignoreBuilder::new("");
    builder.add_line(None, "*.pyc").unwrap();
    let matcher = builder.build().unwrap();
    let paths = [
        ("module.py", false),
        ("module.pyc", true),
        ("src/module.py", false),
        ("src/__pycache__/module.pyc", true),
    ];
    b.iter(|| {
        for (path, ignored) in paths {
            assert_eq!(
                matcher.matched(black_box(path), false).is_ignore(),
                ignored
            );
        }
    });
}

#[bench]
fn match_single_directory_whitelist(b: &mut test::Bencher) {
    let mut builder = GitignoreBuilder::new("");
    builder.add_line(None, "!/.github/").unwrap();
    let matcher = builder.build().unwrap();
    let paths = [
        (".github", true, true),
        (".github", false, false),
        (".github/workflows/check.yml", false, false),
        ("src/module.py", false, false),
        ("src", true, false),
    ];
    b.iter(|| {
        for (path, is_dir, whitelisted) in paths {
            assert_eq!(
                matcher.matched(black_box(path), is_dir).is_whitelist(),
                whitelisted
            );
        }
    });
}

#[bench]
fn match_single_path_whitelist(b: &mut test::Bencher) {
    let mut builder = GitignoreBuilder::new("");
    builder.add_line(None, "!/vendor/typeshed/**/*").unwrap();
    let matcher = builder.build().unwrap();
    let paths = [
        ("vendor/typeshed/stdlib/builtins.pyi", true),
        ("vendor/typeshed/stubs/pkg/__init__.pyi", true),
        ("src/lib.rs", false),
        ("vendor/other/module.py", false),
    ];
    b.iter(|| {
        for (path, whitelisted) in paths {
            assert_eq!(
                matcher.matched(black_box(path), false).is_whitelist(),
                whitelisted
            );
        }
    });
}

#[bench]
fn walk_gitignore_only_parallel_12(b: &mut test::Bencher) {
    let ws = Workspace::new(128, 1, 32, false);
    fs::remove_file(ws.root.join(".ignore")).unwrap();
    fs::remove_file(ws.root.join(".git/info/exclude")).unwrap();
    let builder = ws.builder();
    // The private module is visible without .git/info/exclude.
    let expected = ws.python_files + 1;
    assert_eq!(walk(&builder, 12), expected);
    b.iter(|| assert_eq!(walk(black_box(&builder), 12), expected));
}

#[bench]
fn incremental_gitignore_only(b: &mut test::Bencher) {
    let ws = Workspace::new(16, 4, 4, false);
    fs::remove_file(ws.root.join(".ignore")).unwrap();
    fs::remove_file(ws.root.join(".git/info/exclude")).unwrap();
    let mut matchers = ws.builder().build_matchers();
    for path in &ws.paths {
        assert!(!matchers[0].matched(path, false).is_ignore());
    }
    b.iter(|| {
        for path in &ws.paths {
            assert!(!matchers[0].matched(black_box(path), false).is_ignore());
        }
    });
}

#[bench]
fn walk_selected_python_files(b: &mut test::Bencher) {
    let ws = Workspace::new(128, 1, 32, false);
    let mut overrides = OverrideBuilder::new(&ws.root);
    overrides.add("*.py").unwrap().add("*.pyi").unwrap();
    let mut builder = ws.builder();
    builder.overrides(overrides.build().unwrap());
    assert_eq!(walk(&builder, 12), ws.python_files);
    b.iter(|| assert_eq!(walk(black_box(&builder), 12), ws.python_files));
}
