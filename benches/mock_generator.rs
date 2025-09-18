use std::fs;
use std::io::Write;
use std::path::Path;

const MOCK_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.\n";

/// Controls the depth of directory nesting.
/// For example, with `DIR_DEPTH = 2` files will be placed under `mock_data/dirX/sub0/.../fileY.md`.
const DIR_DEPTH: usize = 3;

/// Controls the number of subdirectories per directory.
const SUBDIRS_PER_DIR: usize = 5;

/// Controls the number of md files per directory.
const FILES_PER_DIR: usize = 10;

struct MockFile {
    path: String,
    content: Vec<String>,
    links: Vec<(String, String)>,
}

fn up_prefix(depth: usize) -> String {
    // Returns a relative prefix to go up from a nested directory to the root of mock_data
    match depth {
        0 => String::new(),
        1 => "../".to_string(),
        n => {
            let mut s = String::with_capacity(3 * n);
            for _ in 0..n {
                s.push_str("../");
            }
            s
        }
    }
}

fn generate_dir(
    base_path: &str,
    current_depth: usize,
    files_to_create: &mut Vec<MockFile>,
    root_links: &mut Vec<(String, String)>,
) {
    // Create files in this directory
    for j in 0..FILES_PER_DIR {
        let file_path = format!("{}/file{}.md", base_path, j);
        let content = vec![format!(
            "# File in {}\n\nThis is file {}.\n\n",
            base_path, j
        )];

        let mut links = Vec::new();
        // Link back to root.
        let up = up_prefix(current_depth);
        links.push(("root".to_string(), format!("{}root.md", up)));

        // Link to parent directory's same-named file, if not at root level
        if current_depth > 0 {
            links.push(("parent_file".to_string(), format!("../file{}.md", j)));
        }

        files_to_create.push(MockFile {
            path: file_path.clone(),
            content,
            links,
        });

        // Add to root links
        let rel_path = &file_path["mock_data/".len()..];
        root_links.push((
            format!("file_{}_{}", current_depth, j),
            format!("./{}", rel_path),
        ));
    }

    if current_depth >= DIR_DEPTH {
        return;
    }

    // Create subdirectories
    for i in 0..SUBDIRS_PER_DIR {
        let subdir_path = format!("{}/subdir{}", base_path, i);
        generate_dir(&subdir_path, current_depth + 1, files_to_create, root_links);
    }
}

pub fn generate() -> std::io::Result<()> {
    let mut files_to_create = Vec::new();
    let mut root_links = Vec::new();

    // Generate directory structure recursively
    generate_dir("mock_data", 0, &mut files_to_create, &mut root_links);

    // Add root file
    files_to_create.push(MockFile {
        path: "mock_data/root.md".to_string(),
        content: vec!["# Root File\n\nThis is the root file.\n\n".to_string()],
        links: root_links,
    });

    // Create files
    if Path::new("mock_data").exists() {
        fs::remove_dir_all("mock_data")?;
    }

    for mock_file in files_to_create {
        let path = Path::new(&mock_file.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = fs::File::create(path)?;

        for line in mock_file.content {
            file.write_all(line.as_bytes())?;
        }

        for (text, link) in mock_file.links {
            writeln!(file, "Link to {}: [{}]({})\n", text, text, link)?;
        }

        for _ in 0..5 {
            file.write_all(MOCK_TEXT.as_bytes())?;
        }
    }

    Ok(())
}
