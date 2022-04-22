// Copyright 2022 Google LLC
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

use crate::common::TestEnvironment;

pub mod common;

#[test]
fn test_sparse() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_success(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    let stdout = test_env.jj_cmd_success(&repo_path, &["sparse", "list"]);
    insta::assert_snapshot!(stdout, @".
");
    std::fs::write(repo_path.join("foo"), "foo").unwrap();
    let stdout = test_env.jj_cmd_success(&repo_path, &["sparse", "remove", "."]);
    insta::assert_snapshot!(stdout, @"Added 0 files, modified 0 files, removed 1 files
");
    assert!(!repo_path.join("foo").exists());
}
