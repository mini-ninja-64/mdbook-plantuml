use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use failure::Error;
use plantuml_backend::{get_extension, get_image_filename, PlantUMLBackend};
use tempfile::{tempdir, TempDir};

/// A trait class for wrapping the actual rendering command
/// Only here to make unit testing the renderer possbile, this is cheating a
/// bit, but the other option is not testing it at all, or partially through
/// integration tests
trait CommandExecutor {
    fn execute(&self, args: &Vec<String>) -> Result<(), Error>;
}

struct RealCommandExecutor;

impl CommandExecutor for RealCommandExecutor {
    fn execute(&self, args: &Vec<String>) -> Result<(), Error> {
        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.arg("/C");
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.arg("-c");
            cmd
        };

        debug!("Executing '{}'", args.join(" "));
        debug!(
            "Working dir '{}'",
            env::current_dir().unwrap_or(PathBuf::from(".")).display()
        );

        let output = cmd
            // We're invoking through the shell, so call it like this:
            // ```sh -c "<args>"```
            // If not done this way sh -c will ignore all data after the first
            // argument (e.g. ```sh -c plantuml source.puml``` will become
            // ```sh -c plantuml```.
            .arg(args.join(" "))
            .output()
            .expect("Failed to start PlantUML application");

        if output.status.success() {
            info!("Successfully generated PlantUML diagrams.");
            debug!(
                "stdout: {}",
                String::from_utf8(output.stdout).unwrap_or(String::from(""))
            );
            debug!(
                "stderr: {}",
                String::from_utf8(output.stderr).unwrap_or(String::from(""))
            );
        } else {
            let msg = format!(
                "Failed to generate PlantUML diagrams, PlantUML exited with code {} ({}).",
                output.status.code().unwrap_or(-9999),
                String::from_utf8(output.stderr).unwrap_or(String::from(""))
            );
            bail!(msg);
        }

        Ok(())
    }
}

pub struct PlantUMLShell {
    plantuml_cmd: String,
    img_root: PathBuf,
    generation_dir: TempDir,
}

/// Invokes PlantUML as a shell/cmd program.
impl PlantUMLShell {
    pub fn new(plantuml_cmd: String, img_root: PathBuf) -> PlantUMLShell {
        PlantUMLShell {
            plantuml_cmd: plantuml_cmd,
            img_root: img_root,
            generation_dir: tempdir().unwrap(),
        }
    }

    /// Get the command line for rendering the given source entry
    fn get_cmd_arguments(&self, file: &PathBuf, extension: &String) -> Result<Vec<String>, Error> {
        let mut args: Vec<String> = Vec::new();
        args.push(self.plantuml_cmd.clone());
        args.push(format!("-t{}", extension));
        args.push(String::from("-nometadata"));
        match file.to_str() {
            Some(s) => args.push(String::from(s)),
            None => {
                bail!("Failed to stringify temporary PlantUML file path.");
            }
        }

        Ok(args)
    }

    /// Create the source and image names for the generation dir with the appropriate extensions
    fn get_filenames(&self, output_file: &PathBuf) -> (PathBuf, PathBuf) {
        let mut puml_image = self.generation_dir.path().to_path_buf();
        puml_image.push(output_file.file_name().unwrap());

        let mut puml_src = puml_image.clone();
        puml_src.set_extension("puml");

        (puml_src, puml_image)
    }

    ///Generate an image file from the given plantuml code.
    fn render_from_string(
        &self,
        plantuml_code: &String,
        command_executor: &dyn CommandExecutor,
    ) -> Result<PathBuf, Error> {
        let output_file = get_image_filename(&self.img_root, &plantuml_code);

        if output_file.exists() {
            info!(
                "Skipping {}, it already exists.",
                output_file.to_string_lossy()
            );
            return Ok(output_file);
        }

        let (puml_src, puml_image) = self.get_filenames(&output_file);
        // Write diagram source file for rendering
        fs::write(puml_src.as_path(), plantuml_code.as_str()).or_else(|e| {
            bail!("Failed to create temp file for inline diagram ({}).", e);
        })?;

        // Render the diagram, PlantUML will create a file with the same base
        // name, and the image extension
        let args = self.get_cmd_arguments(&puml_src, &get_extension(&output_file))?;
        command_executor.execute(&args).or_else(|e| {
            bail!("Failed to render inline diagram ({}).", e);
        })?;

        if !puml_image.exists() {
            bail!(
                format!("PlantUML did not generate an image, did you forget the @startuml, @enduml block ({})?", args.join(" "))
            );
        }

        if let Err(e) = fs::copy(&puml_image, &output_file) {
            bail!(
                "Error copying the generated PlantUML image {} from to {} ({}).",
                puml_image.to_string_lossy(),
                output_file.to_string_lossy(),
                e
            );
        }

        Ok(output_file)
    }
}

impl PlantUMLBackend for PlantUMLShell {
    fn render_from_string(&self, plantuml_code: &String) -> Result<PathBuf, Error> {
        let executor = RealCommandExecutor {};
        PlantUMLShell::render_from_string(self, plantuml_code, &executor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use failure::err_msg;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    struct FakeCommandExecutor {
        error: bool,
        create_file: bool,
    }

    impl CommandExecutor for FakeCommandExecutor {
        fn execute(&self, args: &Vec<String>) -> Result<(), Error> {
            if self.error {
                Err(err_msg("Whoops"))
            } else {
                // Last argument is file name
                if self.create_file {
                    let mut filename = PathBuf::from(args.last().unwrap());
                    let source = &fs::read(filename.clone())?;

                    //Simply copy the contents of source to the output file
                    filename.set_extension("svg");
                    fs::write(filename.as_path(), source)?;
                }
                Ok(())
            }
        }
    }

    #[test]
    fn shell_command_line_arguments() {
        let shell = PlantUMLShell {
            plantuml_cmd: String::from("plantumlcmd"),
            img_root: PathBuf::from(""),
            generation_dir: tempdir().unwrap(),
        };
        let file = PathBuf::from("froboz.puml");
        assert_eq!(
            vec![
                String::from("plantumlcmd"),
                String::from("-tsome_supported_extension"),
                String::from("-nometadata"),
                String::from("froboz.puml")
            ],
            shell
                .get_cmd_arguments(&file, &String::from("some_supported_extension"))
                .unwrap()
        );
    }

    #[test]
    fn command_failure() {
        let output_dir = tempdir().unwrap();
        let shell = PlantUMLShell {
            plantuml_cmd: String::from(""),
            img_root: output_dir.path().to_path_buf(),
            generation_dir: output_dir,
        };

        let executor = FakeCommandExecutor {
            error: true,
            create_file: false,
        };
        match shell.render_from_string(&String::from("@startuml\nA--|>B\n@enduml"), &executor) {
            Ok(_svg) => assert!(false, "Expected the command to fail"),
            Err(e) => assert!(
                e.to_string().contains("Failed to render inline diagram"),
                "Wrong error returned"
            ),
        };
    }

    #[test]
    fn no_image_file_created() {
        let output_dir = tempdir().unwrap();
        let shell = PlantUMLShell {
            plantuml_cmd: String::from(""),
            img_root: output_dir.path().to_path_buf(),
            generation_dir: output_dir,
        };

        let executor = FakeCommandExecutor {
            error: false,
            create_file: false,
        };
        match shell.render_from_string(&String::from("@startuml\nA--|>B\n@enduml"), &executor) {
            Ok(_svg) => assert!(false, "Expected the command to fail"),
            Err(e) => assert!(
                e.to_string().contains("PlantUML did not generate an image"),
                "Wrong error returned"
            ),
        };
    }

    #[test]
    fn returns_image_file_path_on_success() {
        let output_dir = tempdir().unwrap();
        let shell = PlantUMLShell {
            plantuml_cmd: String::from(""),
            // Needs to be different from the generation dir, because else we cannot
            // copy between the two.
            img_root: output_dir.path().to_path_buf(),
            generation_dir: tempdir().unwrap(),
        };

        let executor = FakeCommandExecutor {
            error: false,
            create_file: true,
        };
        let source = String::from("@startuml\nA--|>B\n@enduml");
        match shell.render_from_string(&source, &executor) {
            Ok(img_path) => {
                let raw_source = fs::read(img_path).unwrap();
                let copied_source = String::from_utf8_lossy(&raw_source);
                assert_eq!(source, copied_source)
            }
            Err(e) => assert!(false, e.to_string()),
        };
    }
}
