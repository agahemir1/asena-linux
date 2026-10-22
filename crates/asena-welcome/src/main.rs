use asena_core::i18n::gettext;

fn main() -> anyhow::Result<()> {
    asena_core::logging::init("asena-welcome").map_err(|e| anyhow::anyhow!("{e}"))?;
    asena_core::i18n::init().map_err(|e| anyhow::anyhow!("{e}"))?;
    anyhow::bail!(gettext("This component is not implemented yet."))
}
