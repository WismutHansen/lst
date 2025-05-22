use lazy_static::lazy_static;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use lst::models::ItemStatus;
use lst::storage::{self, markdown};

lazy_static! {
    static ref TEST_ENV: TestEnv = TestEnv::new();
}

struct TestEnv {
    _dir: TempDir,
    config_path: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("create temp dir");
        let config_path = dir.path().join("lst.toml");
        let config = format!("[paths]\ncontent_dir = \"{}\"\n", dir.path().display());
        fs::write(&config_path, config).expect("write config");
        std::env::set_var("LST_CONFIG", &config_path);
        Self { _dir: dir, config_path }
    }
}

fn setup() {
    lazy_static::initialize(&TEST_ENV);
    let lists_dir = storage::get_lists_dir().expect("lists dir");
    if lists_dir.exists() {
        for entry in fs::read_dir(&lists_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                fs::remove_file(path).unwrap();
            }
        }
    }
}

#[test]
fn test_create_list() {
    setup();
    let list = markdown::create_list("create_test").expect("create list");
    let lists_dir = storage::get_lists_dir().unwrap();
    assert!(lists_dir.join("create_test.md").exists());
    assert_eq!(list.metadata.title, "create_test");
}

#[test]
fn test_add_item() {
    setup();
    markdown::create_list("add_item_test").unwrap();
    let item = markdown::add_item("add_item_test", "first item").unwrap();
    assert_eq!(item.text, "first item");
    assert_eq!(item.status, ItemStatus::Todo);
    let list = markdown::load_list("add_item_test").unwrap();
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].text, "first item");
}

#[test]
fn test_mark_done_undone() {
    setup();
    markdown::create_list("status_test").unwrap();
    let item = markdown::add_item("status_test", "task").unwrap();
    let done = markdown::mark_done("status_test", &item.anchor).unwrap();
    assert_eq!(done[0].status, ItemStatus::Done);
    let list = markdown::load_list("status_test").unwrap();
    assert_eq!(list.items[0].status, ItemStatus::Done);
    let undone = markdown::mark_undone("status_test", &item.anchor).unwrap();
    assert_eq!(undone[0].status, ItemStatus::Todo);
    let list2 = markdown::load_list("status_test").unwrap();
    assert_eq!(list2.items[0].status, ItemStatus::Todo);
}

#[test]
fn test_delete_item() {
    setup();
    markdown::create_list("delete_test").unwrap();
    let item1 = markdown::add_item("delete_test", "one").unwrap();
    markdown::add_item("delete_test", "two").unwrap();
    let removed = markdown::delete_item("delete_test", &item1.anchor).unwrap();
    assert_eq!(removed[0].text, "one");
    let list = markdown::load_list("delete_test").unwrap();
    assert_eq!(list.items.len(), 1);
    assert_eq!(list.items[0].text, "two");
}

