//! The `menu_slot_pickup` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// Whether a slot of a registered vanilla or mod menu gives up the stack it holds
    /// (vanilla's `Slot.mayPickup`, which a mod overrides). Asked wherever a click asks the slot
    /// itself: a pickup, a quick move, a number-key or offhand swap, a throw and a double-click
    /// collect, which skips a slot that refuses; a host's `menu.slot_gives` follows it too.
    /// `stack` is the stack the slot holds; an empty slot is not asked. `may_pickup` arrives
    /// `true`: a slot that refuses on its own (a result that is no longer current, an anvil's
    /// unpaid cost, a trade not met, bound armour) is not asked, so a part can refuse a take,
    /// not allow one. Set it `false` and the stack stays in its slot. The menu types, `slot`,
    /// `container` and `context` are `menu_slot_rules`'s: mounts use
    /// `horizon:mount_inventory`, `slot` is the backing index, including the player's inventory
    /// slots inside a container's menu. The player's own inventory menu has no registered type
    /// and is asked on the reserved id `horizon:player_inventory`, for this hook only: its
    /// crafting result is `Result` slot 0, its 2x2 grid `Grid` slots 0-3, and the rest
    /// `Player`'s, the armour 39, 38, 37, 36 from head to feet, the main inventory 9-35, the
    /// hotbar 0-8 and the offhand 40; `context.pos` is `None`. The creative picker is not
    /// asked, and the creative screen sends the server slots, not clicks, so there a refusal is
    /// the client's alone (as vanilla's `mayPickup` is). Runs on server and client
    /// prediction with the read `Host`: it is a query, so it can neither write nor cancel; the
    /// server's answer stands, and a prediction that differs is corrected. A click may ask more
    /// than once for the same slot. Answer from the stack and the host alone, and fast. A part
    /// that fails is logged and its slot gives the stack up.
    #[hook(
        side = "both",
        contract = "listen",
        scope = "menu",
        key = "menu",
        host = "read",
        cancellable = false,
        neoforge = "none (vanilla Slot.mayPickup; NeoForge adds no event or extension for it)"
    )]
    fn menu_slot_pickup(
        menu: &str,
        slot: usize,
        container: MenuContainer,
        stack: &ItemStack,
        context: &MenuSlotContext,
        may_pickup: &mut bool,
    );
}
