use super::*;

impl Game<'_> {
    /// A crafting grid's result was made for a server player and may be changed or cleared:
    /// `pos` is the crafting table or mod crafting block, or None for the inventory grid;
    /// `inputs` is the grid row by row. `result` arrives holding the recipe's result, or what
    /// `native_prepare_item_craft` set for a grid no recipe matched; the mods may set another
    /// stack or `None`. Asked after `native_prepare_item_craft`, when the result was made again.
    pub(in crate::game) fn native_prepare_item_craft_result(
        &mut self,
        player: u128,
        name: &str,
        pos: Option<[i32; 3]>,
        inputs: &[Option<ItemStack>],
        result: &mut Option<ItemStack>,
    ) {
        self.fire_prepare_item_craft_result(player, name, pos, inputs, result);
    }
}
