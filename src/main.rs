extern crate git2;
extern crate regex;
extern crate stopwatch;
extern crate which;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use git2::Repository;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::result::Result;

mod ranking;

#[derive(Serialize, Debug)]
struct CrateInfo {
    name: String,
    version: Option<String>
}

struct State {
    index: Vec<String>,
}

const EXIT_CMD: &str = "\\\\";

const TO_TAKE: usize = 20;

fn walk(repo: &Repository, tree: git2::Tree) -> Result<Vec<String>, git2::Error> {
    let mut names = Vec::new();
    for tree in tree.iter() {
        match tree.kind() {
            Some(git2::ObjectType::Tree) => {
                let subtree_id = tree.id();
                let subtree = repo.find_tree(subtree_id)?;
                let mut subnames = walk(repo, subtree)?;
                names.append(&mut subnames);
            }
            Some(git2::ObjectType::Blob) => {
                let name = tree.name().unwrap();
                names.push(name.to_string());
            }
            _ => (),
        };
    }

    Ok(names)
}

fn find_cargo_repo_path() -> Result<PathBuf, Box<Error>> {
    let cargo_path = which::which("cargo").expect("Cargo binary not found in PATH");
    let cargo_index_path = cargo_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| Some(p.join(Path::new("registry/index/"))))
        .expect("Could not find cargo index folder");

    let git_folders = cargo_index_path.read_dir()?.map(|f| f.unwrap());
    let newest = git_folders
        .max_by_key(|f| f.metadata().unwrap().modified().unwrap())
        .ok_or("No cargo index folders found")?;

    Ok(newest.path())
}

fn build_index() -> Result<Vec<String>, Box<Error>> {
    let mut results = Vec::new();
    let repo = Repository::open(find_cargo_repo_path()?)?;
    let branches = repo.branches(None)?;
    for branch in branches {
        let (branch, _branch_type) = branch?;
        let branch_ref = branch.into_reference();
        let target = branch_ref.target().ok_or("no target oid")?;
        let commit = repo.find_commit(target)?;
        let tree = commit.tree()?;
        let mut names = walk(&repo, tree)?;
        results.append(&mut names);
    }
    Ok(results)
}

fn handle_input(state: &State, input: &str) -> Result<Vec<CrateInfo>, Box<Error>> {
    let parts: Vec<&str> = input.splitn(2, '=').collect();
    let mut items = Vec::new();

    if parts.len() == 2 {
        unimplemented!();
    } else {
        let matches = ranking::search_names(&state.index, input)?;
        let to_take = std::cmp::min(TO_TAKE, matches.len());

        for (name, _score) in &matches[..to_take] {
            items.push(CrateInfo {
                name: name.to_string(),
                version: None
            });
        }
    }

    return Ok(items);
}

fn read_input() -> Result<(), Box<Error>> {
    let index = build_index()?;
    let state = State { index };

    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();

        if trimmed == EXIT_CMD {
            break;
        }

        let results = handle_input(&state, &trimmed)?;
        let ser = serde_json::to_string(&results)?;
        println!("{}", ser);
    }

    Ok(())
}

fn main() {
    // println!(
    //     "Cargo search! Write crate name and hit enter. Write `{}` to quit.",
    //     EXIT_CMD
    // );
    read_input().unwrap();
}
