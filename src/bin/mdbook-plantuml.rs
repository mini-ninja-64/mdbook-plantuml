use clap::{Arg, ArgMatches, Command};
use mdbook::errors::Error as MDBookError;
use mdbook::preprocess::{CmdPreprocessor, Preprocessor};
use mdbook_plantuml::PlantUMLPreprocessor;
use mdbook_plantuml::plantumlconfig::get_plantuml_config;
use std::error::Error;
use std::io;
use std::process;

pub fn make_app() -> Command<'static> {
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    Command::new("mdBook PlantUML preprocessor")
        .version(VERSION)
        .author("Sytse Reitsma")
        .about("An mdbook preprocessor which renders PlantUML code blocks to SVG diagrams")
        .subcommand(
            Command::new("supports")
                .arg(Arg::new("renderer").required(true))
                .about("Check whether a renderer is supported by this preprocessor"),
        )
}

fn main() {
    let matches = make_app().get_matches();

    let preprocessor = PlantUMLPreprocessor;
    if let Some(sub_args) = matches.subcommand_matches("supports") {
        handle_supports(&preprocessor, sub_args);
    } else {
        if let Err(e) = handle_preprocessing(&preprocessor) {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

fn handle_preprocessing(pre: &dyn Preprocessor) -> Result<(), MDBookError> {
    let (ctx, book) = CmdPreprocessor::parse_input(io::stdin())?;

    let cfg = get_plantuml_config(&ctx);

    if cfg.logging_enabled {
        if let Err(e) = setup_logging(&cfg.logging_config) {
                    eprintln!("{}", e);
                    process::exit(2);
                }
    }
    if ctx.mdbook_version != mdbook::MDBOOK_VERSION {
        // We should probably use the `semver` crate to check compatibility
        // here...
        eprintln!(
            "Warning: The {} plugin was built against version {} of mdbook, but we're being \
             called from version {}",
            pre.name(),
            mdbook::MDBOOK_VERSION,
            ctx.mdbook_version
        );
    }
    let processed_book = pre.run(&ctx, book)?;
    serde_json::to_writer(io::stdout(), &processed_book)?;
    Ok(())
}

fn handle_supports(pre: &dyn Preprocessor, sub_args: &ArgMatches) -> ! {
    let renderer = sub_args.value_of("renderer").expect("Required argument");
    let supported = pre.supports_renderer(renderer);

    // Signal whether the renderer is supported by exiting with 1 or 0.
    if supported {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn setup_logging(logging_config_file: &Option<String>) -> Result<(), Box<dyn Error>> {
    use log::LevelFilter;
    use log4rs::append::file::FileAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;
    
    if logging_config_file.is_some() {
        log4rs::init_file(logging_config_file.as_ref().unwrap(), Default::default())?;
    }else {
        let logfile_appender = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
            .build("output.log")?;

        let config = Config::builder()
            .appender(Appender::builder().build("logfile", Box::new(logfile_appender)))
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(LevelFilter::Debug),
            )?;
        log4rs::init_config(config)?;
    }
    Ok(())
}
