use super::*;

impl Game<'_> {
    /// Whether a slot of a registered vanilla or mod menu, or of the player's own inventory
    /// menu, gives up the stack it holds (`Slot.mayPickup`): `menu` is the menu type
    /// (`horizon:player_inventory` for the inventory menu, window 0), `slot` the backing
    /// container index, `stack` the stack in it. `may_pickup` arrives `true` (a slot that
    /// refuses on its own is not asked) and the menu type's parts may set it `false`. A
    /// read-only fire, asked wherever a click asks the slot. `context` is `menu_slot_rules`'s.
    #[inline(never)]
    pub(in crate::game) fn native_menu_slot_pickup(
        &self,
        call: mod_api::__private::MenuSlotPickupCall<'_>,
    ) {
        self.fire_menu_slot_pickup(call);
    }
}
