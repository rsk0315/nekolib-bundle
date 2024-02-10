use std::{
    path::Path,
    process::{Command, Stdio},
};

pub struct Metadata {
    commit: String,
}

impl Metadata {
    pub fn fetch(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let commit = {
            let child = Command::new("git")
                .current_dir(path)
                .args(["log", "-1", "--format=%H"])
                .stdout(Stdio::piped())
                .spawn()?;
            let output = child.wait_with_output()?;
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        let dirty = {
            let child = Command::new("git")
                .current_dir(path)
                .args(["status", "-s"])
                .stdout(Stdio::piped())
                .spawn()?;
            let output = child.wait_with_output()?;
            !output.stdout.is_empty()
        };

        let commit =
            commit.trim_end().to_owned() + if dirty { "-dirty" } else { "" };
        Ok(Self { commit })
    }

    pub fn get_commit(&self) -> &str { &self.commit }
}
