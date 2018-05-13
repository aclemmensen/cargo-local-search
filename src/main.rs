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
use std::path::{Path, PathBuf};
use std::io::{self, Stdin};

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
            _ => ()
        };

    }

    Ok(names)
}

fn build_pattern(query: &str) -> Result<Regex, regex::Error> {
    lazy_static!{
        static ref RE: Regex = Regex::new("(.)").unwrap();
    }

    let enhanced = RE.replace_all(&query, ".*($0)");
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

fn starts_name(index: usize, full_name: &str) -> bool {
    if index == 0 {
        return true
    }
    let full_name = full_name.as_bytes();
    let prev: char = full_name[index - 1].into();
    let curr: char = full_name[index].into();
    prev == '-' || prev == '_' || curr.is_uppercase()
}

fn score_name(name: &String, pattern: &Regex) -> Option<usize> {
    if !pattern.is_match(&name) {
        return None
    }

    let matches = pattern.captures(&name).unwrap();

    let mut score = 0;

    for idx in 0 .. matches.len() - 1 { 
        let m1 = &matches.get(idx).unwrap();
        let m2 = &matches.get(idx + 1).unwrap();
        if starts_name(m1.start(), name) && starts_name(m2.start(), name) {
            continue;
        }
        score = score + (m2.start() - m1.start());
    }

    Some(score)
}

fn search_index(index: &Vec<String>, pattern: &Regex) -> Vec<(String, usize)> {
    let mut results = Vec::new();
    for name in index {
        if let Some(score) = score_name(name, pattern) {
            results.push((name.clone(), score))
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
    println!("Found {} crates in {} ms", index.len(), sw.elapsed_ms());
    sw.restart();
    let pattern = build_pattern("yhm")?;
    let matches = search_index(&index, &pattern);
    
    for (name, score) in &matches {
        println!("{} {}", score, name);
    }
    println!("Found {} matches in {} ms", matches.len(), sw.elapsed_ms());
    Ok(())
}

const EXIT_CMD: &str = "\\\\";

fn handle_input(state: &State, input: &str) -> Result<(), Box<Error>> {
    let parts: Vec<&str> = input.splitn(2, '=').collect();

    if parts.len() == 2 {
        unimplemented!();
    } else {
        let pattern = build_pattern(input)?;
        let matches = search_index(&state.index, &pattern);
        println!("{:?}", matches);
    }

    return Ok(());
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

        handle_input(&state, &trimmed)?;
    }

    Ok(())
}

struct State {
    index: Vec<String>
}

fn main() {
    println!("Cargo search! Write crate name and hit enter. Write `{}` to quit.", EXIT_CMD);
    read_input().unwrap();
    //run().unwrap();
}
