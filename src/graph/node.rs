use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

use cargo::util::errors::CargoResult;

use lazy_static::*;
use semver::Version;

use super::{CommandDetails, SourceKind};
use crate::config::Config;
use crate::path::TargetPath;
use crate::plan::{Invocation, TargetKind};

pub struct Node {
    id: usize,

    package_name: String,
    package_version: Version,

    command: CommandDetails,

    kind: NodeKind,
    source: SourceKind,
    outputs: Vec<TargetPath>,
    links: BTreeMap<TargetPath, TargetPath>,
}

#[derive(Debug, PartialEq)]
enum NodeKind {
    Test,
    Binary,
    Other,
}

impl Node {
    pub fn from_invocation(invocation: &Invocation, config: &Config) -> CargoResult<Self> {
        lazy_static! {
            pub static ref LAST_NODE_ID: AtomicUsize = AtomicUsize::new(0);
        };

        let outputs = {
            invocation
                .outputs
                .iter()
                .map(|path| TargetPath::with_config(&config, &path))
                .collect::<CargoResult<_>>()
                .map_err(|error| error.context("Unable to translate output pathes"))?
        };

        let links = {
            invocation
                .links
                .iter()
                .map(|pair| {
                    Ok((
                        TargetPath::with_config(&config, &pair.0)?,
                        TargetPath::with_config(&config, &pair.1)?,
                    ))
                })
                .collect::<CargoResult<_>>()
                .map_err(|error| error.context("Unable to translate output pathes"))?
        };

        Ok(Self {
            id: LAST_NODE_ID.fetch_add(1, Ordering::Relaxed),
            kind: invocation.into(),

            package_name: invocation.package_name.clone(),
            package_version: invocation.package_version.clone(),

            source: SourceKind::from_invocation(invocation, config)?,
            command: CommandDetails::from_invocation(invocation, config),

            outputs,
            links,
        })
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn get_outputs_iter(&self) -> impl Iterator<Item = &TargetPath> {
        self.outputs.iter()
    }

    pub fn get_links_iter(&self) -> impl Iterator<Item = (&TargetPath, &TargetPath)> {
        self.links.iter()
    }

    pub fn get_exports_iter(&self) -> impl Iterator<Item = &TargetPath> {
        self.get_outputs_iter()
            .chain(self.get_links_iter().map(|pair| pair.0))
    }

    pub fn command(&self) -> &CommandDetails {
        &self.command
    }

    pub fn source(&self) -> &SourceKind {
        &self.source
    }
}

impl From<&Invocation> for NodeKind {
    fn from(invocation: &Invocation) -> Self {
        if invocation.args.contains(&String::from("--test")) {
            return NodeKind::Test;
        }

        if invocation.target_kind.contains(&TargetKind::Bin) {
            return NodeKind::Binary;
        }

        NodeKind::Other
    }
}

impl fmt::Display for Node {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "{} = {}\n[{:?}, id: {}]",
            self.package_name, self.package_version, self.kind, self.id,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::env::current_dir;
    use std::path::PathBuf;

    use cargo::core::Workspace;
    use cargo::util::{CargoResult, Config as CargoConfig};
    use semver::Version;

    use super::*;
    use crate::config::Config;
    use crate::path::TargetPath;
    use crate::plan::Invocation;

    #[test]
    fn node_should_be_constructable() -> CargoResult<()> {
        let cargo_config = CargoConfig::default()?;
        let cargo_ws = Workspace::new(&current_dir()?.join("Cargo.toml"), &cargo_config)?;

        let config = Config::from_cargo_workspace(&cargo_ws)?;

        let invocation = Invocation {
            package_name: "semver".into(),
            package_version: Version::parse("0.9.0")?,

            outputs: vec![
                current_dir()?
                    .join("target")
                    .join("debug")
                    .join("deps")
                    .join("libsemver-f1499887dbdabbd3.rlib"),
            ],

            links: {
                let mut map = BTreeMap::new();

                map.insert(
                    current_dir()?
                        .join("target")
                        .join("debug")
                        .join("libsemver.rlib"),
                    current_dir()?
                        .join("target")
                        .join("debug")
                        .join("deps")
                        .join("libsemver-f1499887dbdabbd3.rlib"),
                );

                map.insert(
                    current_dir()?
                        .join("target")
                        .join("debug")
                        .join("libsemver-copy.rlib"),
                    current_dir()?
                        .join("target")
                        .join("debug")
                        .join("deps")
                        .join("libsemver-f1499887dbdabbd3.rlib"),
                );

                map
            },

            cwd: PathBuf::from("/registry/src/github.com-1ecc6299db9ec823/semver-0.9.0"),

            ..Default::default()
        };

        let node = Node::from_invocation(&invocation, &config)?;

        assert_eq!(node.package_name, String::from("semver"));
        assert_eq!(node.package_version, Version::parse("0.9.0")?);
        assert_eq!(node.kind, NodeKind::Other);

        unsafe {
            assert_eq!(
                node.get_outputs_iter().cloned().collect::<Vec<_>>(),
                vec![TargetPath::from_path(
                    &PathBuf::from("/rust-out")
                        .join("debug")
                        .join("deps")
                        .join("libsemver-f1499887dbdabbd3.rlib"),
                )]
            );
        }

        unsafe {
            assert_eq!(
                node.get_links_iter().collect::<Vec<_>>(),
                vec![
                    (
                        &TargetPath::from_path(
                            &PathBuf::from("/rust-out")
                                .join("debug")
                                .join("libsemver-copy.rlib")
                        ),
                        &TargetPath::from_path(
                            &PathBuf::from("/rust-out")
                                .join("debug")
                                .join("deps")
                                .join("libsemver-f1499887dbdabbd3.rlib")
                        ),
                    ),
                    (
                        &TargetPath::from_path(
                            &PathBuf::from("/rust-out")
                                .join("debug")
                                .join("libsemver.rlib")
                        ),
                        &TargetPath::from_path(
                            &PathBuf::from("/rust-out")
                                .join("debug")
                                .join("deps")
                                .join("libsemver-f1499887dbdabbd3.rlib")
                        ),
                    ),
                ]
            );
        }

        unsafe {
            assert_eq!(
                node.get_exports_iter().cloned().collect::<Vec<_>>(),
                vec![
                    TargetPath::from_path(
                        &PathBuf::from("/rust-out")
                            .join("debug")
                            .join("deps")
                            .join("libsemver-f1499887dbdabbd3.rlib")
                    ),
                    TargetPath::from_path(
                        &PathBuf::from("/rust-out")
                            .join("debug")
                            .join("libsemver-copy.rlib")
                    ),
                    TargetPath::from_path(
                        &PathBuf::from("/rust-out")
                            .join("debug")
                            .join("libsemver.rlib")
                    ),
                ]
            );
        }

        Ok(())
    }
}