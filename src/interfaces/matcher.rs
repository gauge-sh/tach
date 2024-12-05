use crate::core::config::InterfaceConfig;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct CompiledInterface {
    pub from_modules: Vec<Regex>,
    pub expose: Vec<Regex>,
    pub serializable: bool,
}

#[derive(Debug, Clone)]
pub struct CompiledInterfaces {
    interfaces: Vec<CompiledInterface>,
}

impl CompiledInterfaces {
    pub fn build(interfaces: &[InterfaceConfig]) -> Self {
        let compiled = interfaces
            .iter()
            .map(|interface| CompiledInterface {
                serializable: interface.serializable,
                from_modules: interface
                    .from_modules
                    .iter()
                    .map(|pattern| Regex::new(&format!("^{}$", pattern)).unwrap())
                    .collect(),
                expose: interface
                    .expose
                    .iter()
                    .map(|pattern| Regex::new(&format!("^{}$", pattern)).unwrap())
                    .collect(),
            })
            .collect();

        Self {
            interfaces: compiled,
        }
    }

    pub fn matching<'a>(
        &'a self,
        module_path: &'a str,
    ) -> impl Iterator<Item = &'a CompiledInterface> {
        self.interfaces.iter().matching(module_path)
    }

    pub fn serializable(&self) -> impl Iterator<Item = &CompiledInterface> {
        self.interfaces.iter().serializable()
    }
}

// Extension trait for any iterator over CompiledInterface references
pub trait CompiledInterfaceIterExt<'a> {
    fn serializable(self) -> impl Iterator<Item = &'a CompiledInterface>;
    fn matching(self, module_path: &str) -> impl Iterator<Item = &'a CompiledInterface>;
}

impl<'a, I> CompiledInterfaceIterExt<'a> for I
where
    I: Iterator<Item = &'a CompiledInterface>,
{
    fn serializable(self) -> impl Iterator<Item = &'a CompiledInterface> {
        self.filter(|interface| interface.serializable)
    }

    fn matching(self, module_path: &str) -> impl Iterator<Item = &'a CompiledInterface> {
        let module_path = String::from(module_path);
        self.filter(move |interface| {
            interface
                .from_modules
                .iter()
                .any(|re| re.is_match(&module_path))
        })
    }
}
