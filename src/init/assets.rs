//! What ortho carries for `ortho init`: the language reference that the AI
//! instructions hand to agents, and the examples it copies into a project. Copied into
//! `assets/` from LANGUAGE.md and `examples/` by `cargo xtask zed-grammar`.

/// LANGUAGE.md, its links made absolute so they work from wherever it is copied.
pub const LANGUAGE: &str = include_str!("assets/LANGUAGE.md");

/// An example model the user can copy into a project.
pub struct Example {
    /// The folder it is copied to, unless that is taken.
    pub name: &'static str,
    pub label: &'static str,
    /// Its files, by name, with their text.
    pub files: &'static [(&'static str, &'static str)],
}

macro_rules! example {
    ($name:literal, $label:literal, [$($file:literal),* $(,)?]) => {
        Example {
            name: $name,
            label: $label,
            files: &[$(($file, include_str!(concat!("assets/examples/", $name, "/", $file)))),*],
        }
    };
}

pub const EXAMPLES: [Example; 2] = [
    example!(
        "coffee-machine",
        "Coffee machine",
        [
            "deployment.sysml",
            "implementation.sysml",
            "logical.sysml",
            "physical.sysml",
            "process.sysml",
            "use-case.sysml"
        ]
    ),
    example!(
        "survey-drone",
        "Survey drone",
        [
            "deployment.sysml",
            "implementation.sysml",
            "logical.sysml",
            "physical.sysml",
            "process.sysml",
            "use-case.sysml"
        ]
    ),
];

/// The file opened first in a copied example: the use case view, where a model starts.
pub const FIRST_FILE: &str = "use-case.sysml";

/// A folder name not taken yet: `coffee-machine`, else `coffee-machine-2` and so on.
pub fn free_name(name: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(name) {
        return name.to_string();
    }
    (2..).map(|number| format!("{name}-{number}")).find(|candidate| !taken(candidate)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every example's every file is carried, as `cargo xtask zed-grammar` copied them.
    #[test]
    fn carries_every_file_of_every_example() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/init/assets/examples");
        let mut on_disk: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .flat_map(|example| {
                let example = example.unwrap();
                let name = example.file_name().into_string().unwrap();
                std::fs::read_dir(example.path())
                    .unwrap()
                    .map(move |file| format!("{name}/{}", file.unwrap().file_name().into_string().unwrap()))
            })
            .collect();
        on_disk.sort();
        let mut carried: Vec<String> = EXAMPLES
            .iter()
            .flat_map(|example| example.files.iter().map(move |(file, _)| format!("{}/{file}", example.name)))
            .collect();
        carried.sort();
        assert_eq!(carried, on_disk);
        assert!(EXAMPLES.iter().all(|example| example.files.iter().any(|(file, _)| *file == FIRST_FILE)));
    }

    #[test]
    fn free_names() {
        assert_eq!(free_name("a", |_| false), "a");
        assert_eq!(free_name("a", |name| name == "a" || name == "a-2"), "a-3");
    }
}
