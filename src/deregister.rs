// use crate::DeregisterArgs;
// use unic_langid::LanguageIdentifier;

// pub(crate) fn deregister(args: DeregisterArgs) -> Result<(), crate::register::Error> {
//     let lang_id: LanguageIdentifier = args.tag.parse()?;
//     log::info!("Deregistering speller for '{}'...", &lang_id);

//     let keys = crate::register::derive_lang_id_keys(lang_id)?;
//     crate::reg::deregister_langs(&keys)?;

//     crate::refresh::refresh();

//     log::info!("Deregistration complete!");

//     Ok(())
// }
