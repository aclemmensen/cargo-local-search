extern crate git2;
extern crate stopwatch;
extern crate regex;
extern crate which;
#[macro_use] extern crate lazy_static;

use git2::Repository;
use std::result::Result;
use std::error::Error;
use stopwatch::Stopwatch;
use regex::Regex;
use which::which;
use std::path::{Path, PathBuf};

fn walk(repo: &Repository, tree: git2::Tree) -> Result<Vec<String>, git2::Error> {
    let mut names = Vec::new();
    for tree in tree.iter() {
        match tree.kind() {
            Some(git2::ObjectType::Tree) => {
                let subtree_id = tree.id();
                let subtree = repo.find_tree(subtree_id)?;
                let mut subnames = walk(repo, subtree)?;
                names.append(&mut subnames);
            },
            Some(git2::ObjectType::Blob) => {
                let name = tree.name().unwrap();
                names.push(name.to_string());
            },
            Some(_) => (),
            None => ()
        };

    }

    Ok(names)
}

fn build_pattern(query: &str) -> Result<Regex, regex::Error> {
    lazy_static!{
        static ref RE: Regex = Regex::new("(.)").unwrap();
    }

    let enhanced = RE.replace_all(&query, "(.*)$0");
    Regex::new(&enhanced)
}

fn find_cargo_repo_path() -> Result<PathBuf, Box<Error>> {
    let cargo_path = which::which("cargo").expect("Cargo binary not found in PATH");
    let cargo_index_path = cargo_path
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| Some(p.join(Path::new("registry/index/"))))
        .expect("Could not find cargo index folder");

    let git_folders = cargo_index_path.read_dir()?.map(|f| f.unwrap());
    let newest = git_folders.max_by_key(|f| f
                                        .metadata().unwrap()
                                        .modified().unwrap())
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

fn search_index(index: &Vec<String>, pattern: &Regex) -> Vec<(String, usize)> {
    let mut results = Vec::new();
    for name in index {
        if pattern.is_match(&name) {
            let matches = pattern.captures(&name).unwrap();
            let score = matches
                .iter()
                .skip(1)
                .fold(0, |c, m| c + m.map(|m| m.end()-m.start()).unwrap_or(0));

            results.push((name.clone(), score));
        }
    }
    
    results.sort_by(|(n1, s1), (n2, s2)|
        match s1.cmp(&s2) {
            std::cmp::Ordering::Equal => n1.len().cmp(&n2.len()),
            other => other
        });

    results
}

fn run() -> Result<(), Box<Error>> {
    let mut sw = Stopwatch::start_new();
    let index = build_index()?;
    println!("Found {} crates in {}", index.len(), sw.elapsed_ms());
    sw.restart();
    let pattern = build_pattern("hpr")?;
    let matches = search_index(&index, &pattern);
    println!("{:?}", matches);
    println!("Found {} matches in {}", matches.len(), sw.elapsed_ms());
    Ok(())
}

fn main() {
    println!("Hello, world!");
    run().unwrap();
}
