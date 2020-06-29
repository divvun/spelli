pub(crate) fn list() {
    let key = crate::reg::open_key().unwrap();

    println!("Registered spellers:");

    let results = key.values().map(|x| x.unwrap().into_inner()).collect::<Vec<_>>();

    if results.is_empty() {
        println!("  - No spellers registered.");
        return;
    }

    for (name, value) in results {
        println!(" - {} -> {}", name.to_string_lossy(), value);
    }
}
