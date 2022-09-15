use std::path::{Path, PathBuf};

use anyhow::Result;
use askama::Template;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Cli {
    /// The output directory
    #[structopt(short = "o", long = "out", default_value = "out")]
    out: String,
    /// The module tree depth
    #[structopt(short = "d", long = "depth", default_value = "4")]
    depth: u32,
    /// The module tree width at each level
    #[structopt(short = "b", long = "branches", default_value = "4")]
    branches: u32,
}

struct Module {
    children: Vec<Module>,
    // e.g. "a2_a1_a3"
    name: String,
    // e.g. "a1"
    short_name: String,
    // e.g. "a2/a1"
    dir: Option<PathBuf>,
}

impl Module {
    fn new(name: String, short_name: String, dir: Option<PathBuf>) -> Module {
        Module {
            name,
            short_name,
            dir,
            children: vec![],
        }
    }

    fn expand_recurse(&mut self, depth: u32, option: &Cli) {
        if depth == option.depth {
            return;
        }
        let dir = match &self.dir {
            Some(dir) => dir.join(&self.short_name),
            None => PathBuf::from(&self.short_name),
        };
        for index in 0..option.branches {
            let short_name = format!("a{index}");
            let name = format!("{}_{short_name}", self.name);
            let mut module = Module::new(name, short_name, Some(dir.clone()));
            module.expand_recurse(depth + 1, option);
            self.children.push(module);
        }
    }

    fn filename(&self) -> String {
        format!("{}.mjs", self.name)
    }

    fn full_path(&self) -> String {
        match &self.dir {
            Some(dir) => dir.join(self.filename()).display().to_string(),
            None => self.filename(),
        }
    }

    fn relative_path_from_parent(&self) -> String {
        match &self.dir {
            Some(dir) => Path::new(dir.file_name().unwrap())
                .join(self.filename())
                .display()
                .to_string(),
            None => self.filename(),
        }
    }

    fn export_function_name(&self) -> String {
        format!("f_{}", self.name)
    }

    fn function_definition(&self) -> String {
        let mut ops = self
            .children
            .iter()
            .map(|child| format!("{}()", child.export_function_name()))
            .collect::<Vec<_>>();
        ops.push("a".to_string());
        let res = ops.join(" + ");
        format!(
            r#"export function {}() {{
    let a = 1;
    return {res};
}}
"#,
            self.export_function_name()
        )
    }

    fn import_me(&self) -> String {
        format!(
            r#"import {{ {} }} from "./{}""#,
            self.export_function_name(),
            self.relative_path_from_parent()
        )
    }

    fn export(&self, mut builder: webbundle::Builder, option: &Cli) -> Result<webbundle::Builder> {
        match &self.dir {
            Some(dir) => log::debug!("{}", dir.join(self.filename()).display()),
            None => log::debug!("{}", self.filename()),
        };
        let t = ModuleTemplate {
            imports: self
                .children
                .iter()
                .map(|child| child.import_me())
                .collect(),
            function_definition: self.function_definition(),
        };

        let output_dir = match &self.dir {
            Some(dir) => PathBuf::from(&option.out).join(dir),
            None => PathBuf::from(&option.out),
        };

        std::fs::create_dir_all(&output_dir)?;

        let file = PathBuf::from(&option.out).join(self.full_path());
        std::fs::write(file, t.render().unwrap())?;

        builder = builder.exchange((self.full_path(), t.render().unwrap().into_bytes()).into());

        for child in &self.children {
            builder = child.export(builder, option)?;
        }
        Ok(builder)
    }
}

struct Benchmark {
    start_module: Module,
}

impl Benchmark {
    fn new(option: &Cli) -> Benchmark {
        let mut start_module = Module::new("a0".to_string(), "a0".to_string(), None);
        start_module.expand_recurse(0, option);
        Benchmark { start_module }
    }

    fn build(&self, option: &Cli) -> Result<()> {
        self.build_modules(option)?;
        self.build_html(option)
    }

    fn build_modules(&self, option: &Cli) -> Result<()> {
        // Build modules
        let builder = webbundle::Bundle::builder().version(webbundle::Version::VersionB2);
        let builder = self.start_module.export(builder, option)?;

        // Build webbundle
        let webbundle = builder.build()?;
        println!("Build {} modules", webbundle.exchanges().len());
        std::fs::create_dir_all(&option.out)?;
        let f = std::fs::File::create(PathBuf::from(&option.out).join("webbundle.wbn"))?;
        webbundle.write_to(f)?;
        Ok(())
    }

    fn build_html(&self, option: &Cli) -> Result<()> {
        self.build_unbundled_html(option)?;
        self.build_webbundle_html(option)?;
        self.build_index_html(option)
    }

    fn build_unbundled_html(&self, option: &Cli) -> Result<()> {
        let t = BenchmarkTemplate {
            headers: "".to_string(),
            info: format!("option: {option:#?}"),
            modules: vec![],
            start_module: self.start_module.full_path(),
            start_func: self.start_module.export_function_name(),
        };

        std::fs::create_dir_all(&option.out)?;
        let file = PathBuf::from(&option.out).join("unbundled.html");
        std::fs::write(file, t.render().unwrap())?;
        Ok(())
    }

    fn build_webbundle_html(&self, option: &Cli) -> Result<()> {
        let t = BenchmarkTemplate {
            headers: r#"<script type="webbundle"> { "source": "webbundle.wbn", "scopes": ["."] } </script>"#.to_string(),
            info: format!("option: {option:#?}"),
            modules: vec![],
            start_module: self.start_module.full_path(),
            start_func: self.start_module.export_function_name(),
        };

        std::fs::create_dir_all(&option.out)?;
        let file = PathBuf::from(&option.out).join("webbundle.html");
        std::fs::write(file, t.render().unwrap())?;
        Ok(())
    }

    fn build_index_html(&self, option: &Cli) -> Result<()> {
        let t = IndexTemplate {
            info: format!("option: {option:#?}"),
            benchmarks: vec!["unbundled".to_string(), "webbundle".to_string()],
        };

        std::fs::create_dir_all(&option.out)?;
        let file = PathBuf::from(&option.out).join("index.html");
        std::fs::write(file, t.render().unwrap())?;
        Ok(())
    }
}

#[derive(Template)]
#[template(path = "module.html")]
struct ModuleTemplate {
    imports: Vec<String>,
    function_definition: String,
}

#[derive(Template)]
#[template(path = "benchmark.html")]
struct BenchmarkTemplate {
    headers: String,
    info: String,
    modules: Vec<String>,
    start_module: String,
    start_func: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    info: String,
    benchmarks: Vec<String>,
}

fn main() -> Result<()> {
    env_logger::init();
    let args = Cli::from_args();
    let benchmark = Benchmark::new(&args);
    benchmark.build(&args)
}
