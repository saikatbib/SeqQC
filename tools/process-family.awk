{ parent[$1] = $2 }
END {
    seen[root] = 1
    changed = 1
    while (changed) {
        changed = 0
        for (pid in parent) {
            if (seen[parent[pid]] && !seen[pid]) {
                seen[pid] = 1
                changed = 1
            }
        }
    }
    # Membership lookups can create false-valued keys in awk. Emit only members.
    for (pid in seen) if (seen[pid]) print pid
}
