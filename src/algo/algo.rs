
fn reachable(root: u64, read_map: &HashMap<u64, HashMap<usize, u64>>) -> HashSet<u64> {
    let mut stack = Vec::new();
    let mut seen = HashSet::new();

    stack.push(root);
    // seen.insert(root);

    while let Some(u) = stack.pop() {
        if let Some(vs) = read_map.get(&u) {
            for &v in vs.values() {
                if seen.insert(v) {
                    stack.push(v);
                }
            }
        }
    }

    seen
}

fn is_irreflexive(read_map: &HashMap<u64, HashMap<usize, u64>>) -> bool {
    for &e in read_map.keys() {
        let r = reachable(e, &read_map);
        if r.contains(&e) {
            println!("found {} {:?}", e, r);
            return false;
        }
    }
    return true;
}
