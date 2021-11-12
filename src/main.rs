use chrono::NaiveDateTime;

use git2::{Commit, Object, ObjectType, Oid, Repository, Tree};
use indicatif::ProgressBar;
use thread_local::ThreadLocal;
use std::{env, sync::{Arc, mpsc::channel}};

use anyhow::Result;

use rayon::prelude::*;
use threadpool::ThreadPool;

fn main() -> Result<()> {
  let args: Vec<_> = env::args().collect();
  let repo_path = args[0].to_owned();

  let repo = Repository::open(&repo_path)?;
  let mut revwalk = repo.revwalk()?;
  revwalk.push_head()?;

  let mut oids = Vec::new();

  for oid in revwalk {
    oids.push(oid?)
  }

  let mut diffs = Vec::new();

  let mut last_tree: Option<Tree> = None;
  let mut last_oid = None;
  oids.reverse();
  for oid in oids {
    let object = repo.find_object(oid, Some(ObjectType::Commit))?;
    let commit = object.as_commit();

    match commit {
      Some(commit) => {
        // let when = NaiveDateTime::from_timestamp(commit.time().seconds(), 0);
        // //                println!("{} {} {:?}", commit.author(), when, commit.summary());

        let tree = commit.tree()?;
        diffs.push(DiffInfo {
          oid,
          last_oid,
          when: NaiveDateTime::from_timestamp(commit.time().seconds(), 0),
          summary: commit.summary().unwrap().to_owned(),
          author: commit.author().email().unwrap().to_owned(),
        });

        // let diff = repo.diff_tree_to_tree(last_tree.as_ref(), Some(&tree), None)?;
        // let stats = diff.stats()?;
        // //                println!("  {} changed, {} added, {} deleted", stats.files_changed(), stats.insertions(), stats.deletions());

        last_tree = Some(tree);
        last_oid = Some(oid);
      }
      None => println!("{:?}", oid.to_string()),
    }
  }


  let pool = ThreadPool::new(16);
  let (tx, rx) = channel();

  let tl = Arc::new(ThreadLocal::new());
  let repo_path = Arc::new(repo_path);

  let count = diffs.len();
  let progress = ProgressBar::new(count as u64);
  for diff in diffs {
    let tx = tx.clone();
    let tl = tl.clone();
    let progress = progress.clone();
    let repo_path = repo_path.clone();
    pool.execute(move || {
      let repo = tl.get_or(|| Repository::open(repo_path.as_str()).unwrap());

      let object = repo.find_object(diff.oid, Some(ObjectType::Commit)).unwrap();
      let commit = object.as_commit().unwrap();
      let tree = commit.tree().unwrap();

      let prev_tree = match diff.last_oid {
        Some(oid) => {
          let prev_object = repo.find_object(oid, Some(ObjectType::Commit)).unwrap();
          let commit = prev_object.as_commit().unwrap();
          let tree = commit.tree().unwrap();
          Some(tree)
        },
        None => None
      };

      repo.diff_tree_to_tree(prev_tree.as_ref(), Some(&tree), None).unwrap();
      progress.inc(1);

//      println!("Diff {}", diff.author);
      tx.send(diff).unwrap()
    })
  }

  for r in rx.iter().take(count) {

  }
  progress.finish();



  // // Do diffs in parallel.
  // diffs.par_iter().map(|d| {
  //   let last_tree = match d.last_oid {
  //     Some(oid) => {
  //       let commit = repo.find_object(oid, Some(ObjectType::Commit)).unwrap().as_commit();
  //       match commit {
  //         None => Ok(None),
  //         Some(commit) => Ok(Some(commit.tree().unwrap()))
  //       }
  //     },
  //     None => Ok(None)
  //   };

  //   let this_commit = repo.find_object(d.oid, Some(ObjectType::Commit))?;

  //   Ok(None)
  //   // let object = repo.find_object(d.last_oid)

  //   // repo.diff_tree_to_tree(d.last_tree, Some(&d.this_tree), None)
  // });


  Ok(())
}


struct DiffInfo {
  when: NaiveDateTime,
  summary: String,
  author: String,
  oid: Oid,
  last_oid: Option<Oid>
  // last_tree: Option<Tree<'a>>,
  // this_tree: Tree<'a>
}