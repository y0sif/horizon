//! The `prepare_item_craft_result` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A crafting grid's result was made for a server player, and may be changed or cleared
    /// (Paper's `PrepareItemCraftEvent` with `setResult`): `pos` is the crafting table or the
    /// mod crafting block, or None for the inventory grid. `inputs` is the grid row by row
    /// (4 cells for an inventory's grid, 9 for a crafting table's; `None` for an empty cell).
    /// `result` arrives holding the recipe's result, or what `prepare_item_craft` set for a
    /// grid no recipe matched, or `None`; each listener sees what the one before it left. Set
    /// another stack, or `None` to leave the result slot empty: the player is shown, and
    /// takes, what the last listener left, and a take of a replaced result uses up the grid as
    /// the recipe's own does. Heard after `prepare_item_craft`, once each time the result is
    /// made again: when the grid changes (a click, a recipe-book placement, a take, a mod's
    /// write) and when a table opens with a grid, not on a click that leaves the grid alone.
    /// A shift-click that crafts several times is heard after the click, not between its
    /// crafts, which repeat only while the result stays the same item. Guest state is not
    /// swapped in. Server-side.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "none (NeoForge has no such event)",
        paper = "PrepareItemCraftEvent"
    )]
    fn prepare_item_craft_result();
}
