use crate::flags::{HiArgs, SearchMode};

pub(crate) fn write(_args: &HiArgs) -> anyhow::Result<()> {
    anyhow::bail!("indexing not enabled in this build of ripgrep")
}

pub(crate) fn read(_args: &HiArgs, _mode: SearchMode) -> anyhow::Result<bool> {
    anyhow::bail!("indexing not enabled in this build of ripgrep")
}
