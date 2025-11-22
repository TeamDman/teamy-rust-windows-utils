use crate::cli::to_args::ToArgs;
use crate::explorer::context_menu::get_context_menu_entries;
use arbitrary::Arbitrary;
use clap::Args;
use eyre::Result;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Args, Debug, PartialEq)]
pub struct EntryListArgs {
    #[arg(long)]
    pub r#for: PathBuf,
}

impl<'a> Arbitrary<'a> for EntryListArgs {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let mut p = PathBuf::arbitrary(u)?;
        if p.as_os_str().is_empty() {
            p = PathBuf::from(".");
        }
        Ok(EntryListArgs { r#for: p })
    }
}

impl ToArgs for EntryListArgs {
    fn to_args(&self) -> Vec<OsString> {
        let mut args = Vec::new();
        args.push("--for".into());
        args.push(self.r#for.clone().into());
        args
    }
}

impl EntryListArgs {
    pub fn invoke(self) -> Result<()> {
        let path = self.r#for.canonicalize()?;
        let path_str = path.to_string_lossy();

        println!("Inspecting context menu for: {}", path_str);

        unsafe {
            let entries = get_context_menu_entries(&path)?;
            print_entries(&entries, 0);
        }

        Ok(())
    }
}

fn print_entries(entries: &[crate::explorer::context_menu::ContextMenuEntry], depth: usize) {
    let indent = "  ".repeat(depth);
    for entry in entries {
        if entry.is_separator {
            println!("{}----------------", indent);
        } else {
            println!(
                "{}[{}] '{}' (Verb: {})",
                indent, entry.id, entry.label, entry.verb
            );
            if !entry.sub_items.is_empty() {
                print_entries(&entry.sub_items, depth + 1);
            }
        }
    }
}
