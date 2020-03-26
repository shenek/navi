use crate::filesystem;
use crate::finder;
use crate::git;
use crate::structures::finder::{Opts as FinderOpts, SuggestionType};
use anyhow::Context;
use anyhow::Error;
use git2::Repository;
use std::fs;
use std::io::Write;
use walkdir::WalkDir;

pub fn browse() -> Result<(), Error> {
    let repo_path_str = format!("{}/featured", filesystem::tmp_path_str()?);

    // The dir might not exist which would throw an error. But here we don't care about that
    // since our purpose is to make space for a new directory. Potential other errors
    // (e.g due to lack of permission) will cause an error during dir creation.
    let _ = filesystem::remove_dir(&repo_path_str);
    filesystem::create_dir(&repo_path_str)?;

    let repo_url = "https://github.com/denisidoro/cheats";
    Repository::clone(repo_url, &repo_path_str)
        .with_context(|| format!("Failed to clone `{}`", repo_url))?;

    let repos = fs::read_to_string(format!("{}/featured_repos.txt", &repo_path_str))
        .context("Unable to fetch featured repositories")?;

    let opts = FinderOpts {
        column: Some(1),
        ..Default::default()
    };

    let (repo, _) = finder::call(opts, repos).context("Failed to get repo URL from finder")?;

    filesystem::remove_dir(&repo_path_str)?;

    add(repo)
}

pub fn add(uri: String) -> Result<(), Error> {
    let (actual_uri, user, repo) = git::meta(uri.as_str());

    let cheat_path_str = filesystem::pathbuf_to_string(filesystem::cheat_pathbuf()?)?;
    let tmp_path_str = filesystem::tmp_path_str()?;
    let tmp_path_str_with_trailing_slash = format!("{}/", &tmp_path_str);

    let _ = filesystem::remove_dir(&tmp_path_str);
    filesystem::create_dir(&tmp_path_str)?;

    eprintln!("Cloning {} into {}...\n", &actual_uri, &tmp_path_str);

    Repository::clone(actual_uri.as_str(), &tmp_path_str)
        .with_context(|| format!("Failed to clone `{}`", actual_uri))?;

    let all_files = WalkDir::new(&tmp_path_str)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_str().unwrap_or("").to_string())
        .filter(|e| e.ends_with(".cheat"))
        .map(|e| e.replace(&tmp_path_str_with_trailing_slash, ""))
        .collect::<Vec<String>>()
        .join("\n");

    let opts = FinderOpts {
        suggestion_type: SuggestionType::MultipleSelections,
        preview: Some(format!("cat '{}/{{}}'", tmp_path_str)),
        header: Some(
            "Select the cheatsheets you want to import with <TAB> then hit <Enter>".to_string(),
        ),
        preview_window: Some("right:30%".to_string()),
        ..Default::default()
    };

    let (files, _) =
        finder::call(opts, all_files).context("Failed to get cheatsheet files from finder")?;

    for f in files.split('\n') {
        let from = format!("{}/{}", tmp_path_str, f).replace("./", "");
        let to_folder = format!("{}/{}__{}", cheat_path_str, user, repo).replace("./", "");
        let filename = f.replace("./", "").replace("/", "__");
        let to = format!("{}/{}", to_folder, filename);
        fs::create_dir_all(to_folder).unwrap_or(());
        fs::copy(&from, &to).with_context(|| format!("Failed to copy `{}` to `{}`", from, to))?;
    }

    filesystem::remove_dir(&tmp_path_str)?;

    eprintln!("The following .cheat files were imported successfully:\n{}\n\nThey are now located at {}\n\nPlease run navi again to check the results.", files, cheat_path_str);

    Ok(())
}
