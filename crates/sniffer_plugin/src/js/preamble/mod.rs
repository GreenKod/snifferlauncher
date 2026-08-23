pub const IPC_PREAMBLE: &str = concat!(
    include_str!("ipc.js"),
    "\n",
    include_str!("vault.js"),
    "\n",
    include_str!("system.js"),
    "\n",
    include_str!("timers.js"),
    "\n",
    include_str!("shared_view.js")
);
