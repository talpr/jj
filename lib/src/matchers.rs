// Copyright 2020 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap, HashSet};

use maplit::hashset;

use crate::repo_path::{RepoPath, RepoPathComponent};

#[derive(PartialEq, Eq, Debug)]
pub enum Visit {
    /// Everything in the directory is *guaranteed* to match, no need to check
    /// descendants
    AllRecursively,
    Some {
        dirs: VisitDirs,
        files: VisitFiles,
    },
}

impl Visit {
    pub fn nothing() -> Self {
        Self::Some {
            dirs: VisitDirs::Set(hashset! {}),
            files: VisitFiles::Set(hashset! {}),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum VisitDirs {
    All,
    Set(HashSet<RepoPathComponent>),
}

#[derive(PartialEq, Eq, Debug)]
pub enum VisitFiles {
    All,
    Set(HashSet<RepoPathComponent>),
}

pub trait Matcher {
    fn matches(&self, file: &RepoPath) -> bool;
    fn visit(&self, dir: &RepoPath) -> Visit;
}

#[derive(PartialEq, Eq, Debug)]
pub struct NothingMatcher;

impl Matcher for NothingMatcher {
    fn matches(&self, _file: &RepoPath) -> bool {
        false
    }

    fn visit(&self, _dir: &RepoPath) -> Visit {
        Visit::nothing()
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct EverythingMatcher;

impl Matcher for EverythingMatcher {
    fn matches(&self, _file: &RepoPath) -> bool {
        true
    }

    fn visit(&self, _dir: &RepoPath) -> Visit {
        Visit::AllRecursively
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct FilesMatcher {
    files: HashSet<RepoPath>,
    dirs: Dirs,
}

impl FilesMatcher {
    pub fn new(files: HashSet<RepoPath>) -> Self {
        let mut dirs = Dirs::new();
        for f in &files {
            dirs.add_file(f);
        }
        FilesMatcher { files, dirs }
    }
}

impl Matcher for FilesMatcher {
    fn matches(&self, file: &RepoPath) -> bool {
        self.files.contains(file)
    }

    fn visit(&self, dir: &RepoPath) -> Visit {
        let dirs = self.dirs.get_dirs(dir);
        let files = self.dirs.get_files(dir);
        Visit::Some {
            dirs: VisitDirs::Set(dirs),
            files: VisitFiles::Set(files),
        }
    }
}

pub struct PrefixMatcher {
    prefixes: BTreeSet<RepoPath>,
    dirs: Dirs,
}

impl PrefixMatcher {
    pub fn new(prefixes: &[RepoPath]) -> Self {
        let prefixes = prefixes.iter().cloned().collect();
        let mut dirs = Dirs::new();
        for prefix in &prefixes {
            dirs.add_dir(prefix);
            if !prefix.is_root() {
                dirs.add_file(prefix);
            }
        }
        PrefixMatcher { prefixes, dirs }
    }
}

impl Matcher for PrefixMatcher {
    fn matches(&self, file: &RepoPath) -> bool {
        let components = file.components();
        // TODO: Make Dirs a trie instead, so this can just walk that trie.
        for i in 0..components.len() + 1 {
            let prefix = RepoPath::from_components(components[0..i].to_vec());
            if self.prefixes.contains(&prefix) {
                return true;
            }
        }
        false
    }

    fn visit(&self, dir: &RepoPath) -> Visit {
        if self.matches(dir) {
            Visit::AllRecursively
        } else {
            let dirs = self.dirs.get_dirs(dir);
            let files = self.dirs.get_files(dir);
            Visit::Some {
                dirs: VisitDirs::Set(dirs),
                files: VisitFiles::Set(files),
            }
        }
    }
}

/// Keeps track of which subdirectories and files of each directory need to be
/// visited.
#[derive(PartialEq, Eq, Debug)]
struct Dirs {
    dirs: HashMap<RepoPath, HashSet<RepoPathComponent>>,
    files: HashMap<RepoPath, HashSet<RepoPathComponent>>,
}

impl Dirs {
    fn new() -> Self {
        Dirs {
            dirs: HashMap::new(),
            files: HashMap::new(),
        }
    }

    fn add_dir(&mut self, dir: &RepoPath) {
        let mut dir = dir.clone();
        let mut maybe_child = None;
        loop {
            let was_present = self.dirs.contains_key(&dir);
            let children = self.dirs.entry(dir.clone()).or_default();
            if let Some(child) = maybe_child {
                children.insert(child);
            }
            if was_present {
                break;
            }
            match dir.split() {
                None => break,
                Some((new_dir, new_child)) => {
                    maybe_child = Some(new_child.clone());
                    dir = new_dir;
                }
            };
        }
    }

    fn add_file(&mut self, file: &RepoPath) {
        let (dir, basename) = file
            .split()
            .unwrap_or_else(|| panic!("got empty filename: {:?}", file));
        self.add_dir(&dir);
        self.files.entry(dir).or_default().insert(basename.clone());
    }

    fn get_dirs(&self, dir: &RepoPath) -> HashSet<RepoPathComponent> {
        self.dirs.get(dir).cloned().unwrap_or_default()
    }

    fn get_files(&self, dir: &RepoPath) -> HashSet<RepoPathComponent> {
        self.files.get(dir).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repo_path::{RepoPath, RepoPathComponent};

    #[test]
    fn test_dirs_empty() {
        let dirs = Dirs::new();
        assert_eq!(dirs.get_dirs(&RepoPath::root()), hashset! {});
    }

    #[test]
    fn test_dirs_root() {
        let mut dirs = Dirs::new();
        dirs.add_dir(&RepoPath::root());
        assert_eq!(dirs.get_dirs(&RepoPath::root()), hashset! {});
    }

    #[test]
    fn test_dirs_dir() {
        let mut dirs = Dirs::new();
        dirs.add_dir(&RepoPath::from_internal_string("dir"));
        assert_eq!(
            dirs.get_dirs(&RepoPath::root()),
            hashset! {RepoPathComponent::from("dir")}
        );
    }

    #[test]
    fn test_dirs_file() {
        let mut dirs = Dirs::new();
        dirs.add_file(&RepoPath::from_internal_string("dir/file"));
        assert_eq!(
            dirs.get_dirs(&RepoPath::root()),
            hashset! {RepoPathComponent::from("dir")}
        );
        assert_eq!(dirs.get_files(&RepoPath::root()), hashset! {});
    }

    #[test]
    fn test_nothingmatcher() {
        let m = NothingMatcher;
        assert!(!m.matches(&RepoPath::from_internal_string("file")));
        assert!(!m.matches(&RepoPath::from_internal_string("dir/file")));
        assert_eq!(m.visit(&RepoPath::root()), Visit::nothing());
    }

    #[test]
    fn test_filesmatcher_empty() {
        let m = FilesMatcher::new(hashset! {});
        assert!(!m.matches(&RepoPath::from_internal_string("file")));
        assert!(!m.matches(&RepoPath::from_internal_string("dir/file")));
        assert_eq!(m.visit(&RepoPath::root()), Visit::nothing());
    }

    #[test]
    fn test_filesmatcher_nonempty() {
        let m = FilesMatcher::new(hashset! {
            RepoPath::from_internal_string("dir1/subdir1/file1"),
            RepoPath::from_internal_string("dir1/subdir1/file2"),
            RepoPath::from_internal_string("dir1/subdir2/file3"),
            RepoPath::from_internal_string("file4"),
        });

        assert_eq!(
            m.visit(&RepoPath::root()),
            Visit::Some {
                dirs: VisitDirs::Set(hashset! {RepoPathComponent::from("dir1")}),
                files: VisitFiles::Set(hashset! {RepoPathComponent::from("file4")}),
            }
        );
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("dir1")),
            Visit::Some {
                dirs: VisitDirs::Set(
                    hashset! {RepoPathComponent::from("subdir1"), RepoPathComponent::from("subdir2")}
                ),
                files: VisitFiles::Set(hashset! {}),
            }
        );
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("dir1/subdir1")),
            Visit::Some {
                dirs: VisitDirs::Set(hashset! {}),
                files: VisitFiles::Set(
                    hashset! {RepoPathComponent::from("file1"), RepoPathComponent::from("file2")}
                ),
            }
        );
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("dir1/subdir2")),
            Visit::Some {
                dirs: VisitDirs::Set(hashset! {}),
                files: VisitFiles::Set(hashset! {RepoPathComponent::from("file3")}),
            }
        );
    }

    #[test]
    fn test_prefixmatcher_empty() {
        let m = PrefixMatcher::new(&[]);
        assert!(!m.matches(&RepoPath::from_internal_string("file")));
        assert!(!m.matches(&RepoPath::from_internal_string("dir/file")));
        assert_eq!(m.visit(&RepoPath::root()), Visit::nothing());
    }

    #[test]
    fn test_prefixmatcher_root() {
        let m = PrefixMatcher::new(&[RepoPath::root()]);
        // Matches all files
        assert!(m.matches(&RepoPath::from_internal_string("file")));
        assert!(m.matches(&RepoPath::from_internal_string("dir/file")));
        // Visits all directories
        assert_eq!(m.visit(&RepoPath::root()), Visit::AllRecursively);
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("foo/bar")),
            Visit::AllRecursively
        );
    }

    #[test]
    fn test_prefixmatcher_single_prefix() {
        let m = PrefixMatcher::new(&[RepoPath::from_internal_string("foo/bar")]);

        // Parts of the prefix should not match
        assert!(!m.matches(&RepoPath::from_internal_string("foo")));
        assert!(!m.matches(&RepoPath::from_internal_string("bar")));
        // A file matching the prefix exactly should match
        assert!(m.matches(&RepoPath::from_internal_string("foo/bar")));
        // Files in subdirectories should match
        assert!(m.matches(&RepoPath::from_internal_string("foo/bar/baz")));
        assert!(m.matches(&RepoPath::from_internal_string("foo/bar/baz/qux")));
        // Sibling files should not match
        assert!(!m.matches(&RepoPath::from_internal_string("foo/foo")));
        // An unrooted "foo/bar" should not match
        assert!(!m.matches(&RepoPath::from_internal_string("bar/foo/bar")));

        // The matcher should only visit directory foo/ in the root (file "foo"
        // shouldn't be visited)
        assert_eq!(
            m.visit(&RepoPath::root()),
            Visit::Some {
                dirs: VisitDirs::Set(hashset! {RepoPathComponent::from("foo")}),
                files: VisitFiles::Set(hashset! {}),
            }
        );
        // Inside parent directory "foo/", both subdirectory "bar" and file "bar" may
        // match
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("foo")),
            Visit::Some {
                dirs: VisitDirs::Set(hashset! {RepoPathComponent::from("bar")}),
                files: VisitFiles::Set(hashset! {RepoPathComponent::from("bar")}),
            }
        );
        // Inside a directory that matches the prefix, everything matches recursively
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("foo/bar")),
            Visit::AllRecursively
        );
        // Same thing in subdirectories of the prefix
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("foo/bar/baz")),
            Visit::AllRecursively
        );
        // Nothing in directories that are siblings of the prefix can match, so don't
        // visit
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("bar")),
            Visit::nothing()
        );
    }

    #[test]
    fn test_prefixmatcher_nested_prefixes() {
        let m = PrefixMatcher::new(&[
            RepoPath::from_internal_string("foo"),
            RepoPath::from_internal_string("foo/bar/baz"),
        ]);

        assert!(m.matches(&RepoPath::from_internal_string("foo")));
        assert!(!m.matches(&RepoPath::from_internal_string("bar")));
        assert!(m.matches(&RepoPath::from_internal_string("foo/bar")));
        // Matches because the the "foo" pattern matches
        assert!(m.matches(&RepoPath::from_internal_string("foo/baz/foo")));

        assert_eq!(
            m.visit(&RepoPath::root()),
            Visit::Some {
                dirs: VisitDirs::Set(hashset! {RepoPathComponent::from("foo")}),
                files: VisitFiles::Set(hashset! {RepoPathComponent::from("foo")}),
            }
        );
        // Inside a directory that matches the prefix, everything matches recursively
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("foo")),
            Visit::AllRecursively
        );
        // Same thing in subdirectories of the prefix
        assert_eq!(
            m.visit(&RepoPath::from_internal_string("foo/bar/baz")),
            Visit::AllRecursively
        );
    }
}
