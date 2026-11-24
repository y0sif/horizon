use super::*;

impl Game<'_> {
    /// A smithing table's result was made for a server player at `pos` and may be changed or
    /// cleared: `inputs` is the template, the base and the addition. `result` arrives holding
    /// the recipe's result, or what `native_prepare_smithing` set where no recipe matched; the
    /// mods may set another stack or `None`. Asked after `native_prepare_smithing`.
    pub(in crate::game) fn native_prepare_smithing_result(
        &mut self,
        player: u128,
        name: &str,
        pos: [i32; 3],
        inputs: &[Option<ItemStack>],
        result: &mut Option<ItemStack>,
    ) {
        self.fire_prepare_smithing_result(player, name, pos, inputs, result);
    }
}
