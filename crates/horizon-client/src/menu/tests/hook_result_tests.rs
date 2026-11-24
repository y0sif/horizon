//! A crafting result a mod leaves in the result slot (`prepare_item_craft`,
//! or a patched `Menu::recompute`) has no recipe behind it: its take still
//! uses up the grid.

use super::*;

/// Paper's `PrepareItemCraftEvent` keeps vanilla's take for a result it sets:
/// one of each occupied slot, with each item's own remainder. A hook's result
/// has no recipe to ask for remainders, so `Menu::on_take` falls back to the
/// items' own; taking it must not leave the grid untouched for another free
/// take.
#[test]
fn a_hook_result_take_uses_up_the_grid_and_its_remainders() {
    struct Tool;
    impl crate::crafting::CraftingRemainder for Tool {
        fn crafting_remainder(&self, stack: &ItemStack) -> Option<ItemStack> {
            (&*stack.item == "test:tool" && stack.damage < 4).then(|| ItemStack {
                count: 1,
                damage: stack.damage + 1,
                ..stack.clone()
            })
        }
    }
    bare_env(|base| {
        let recipes = Recipes::empty();
        let env = Env {
            recipes: &recipes,
            crafting_remainder: Some(&Tool),
            ..*base
        };
        let cases: [(Kind, &[usize]); 3] = [
            (Kind::Inventory, &[1, 4]),
            (Kind::Crafting, &[1, 5, 9]),
            (mod_crafting_table(), &[1, 5, 9]),
        ];
        for (kind, cells) in cases {
            let mut menu = Menu::new(kind);
            menu.set_slot(&env, cells[0], Some(ItemStack::new("minecraft:stick", 2)));
            menu.set_slot(
                &env,
                cells[1],
                Some(ItemStack {
                    damage: 0,
                    ..ItemStack::new("test:tool", 1)
                }),
            );
            if let Some(&cell) = cells.get(2) {
                menu.set_slot(&env, cell, Some(ItemStack::new("minecraft:dirt", 1)));
            }
            // What a hook, or a patched `recompute`, leaves: no recipe matches.
            menu.set_slot(&env, 0, Some(ItemStack::new("minecraft:diamond", 1)));
            menu.click(&env, 0, 0, Input::Pickup).unwrap();
            assert_eq!(menu.carried, Some(ItemStack::new("minecraft:diamond", 1)));
            assert!(menu.slot(0).is_none(), "the result is taken, not left");
            assert_eq!(menu.slot(cells[0]).map(|s| s.count), Some(1));
            assert_eq!(
                menu.slot(cells[1]).map(|s| (s.damage, s.count)),
                Some((1, 1)),
                "the tool's own remainder"
            );
            if let Some(&cell) = cells.get(2) {
                assert!(menu.slot(cell).is_none());
            }
            // A click on the emptied result takes nothing more.
            menu.carried = None;
            menu.click(&env, 0, 0, Input::Pickup).unwrap();
            assert!(menu.carried.is_none());
            assert_eq!(menu.slot(cells[0]).map(|s| s.count), Some(1));
            // The hook answers again for the shorter grid: the next take uses
            // one more of each occupied slot.
            menu.set_slot(&env, 0, Some(ItemStack::new("minecraft:diamond", 1)));
            menu.click(&env, 0, 0, Input::Pickup).unwrap();
            assert!(menu.slot(0).is_none());
            assert!(menu.slot(cells[0]).is_none());
            assert_eq!(
                menu.slot(cells[1]).map(|s| (s.damage, s.count)),
                Some((2, 1))
            );
        }
    });
}

/// A result the mods set in place of the recipe's (`prepare_item_craft_result`) is current
/// while the grid stands, also where a mod's crafting recipes make every take check the result
/// against the recipe's. Its take uses up the grid, and the result made from what is left is
/// the recipe's again, for the mods to be asked about.
#[test]
fn a_replaced_result_is_taken_while_mod_recipes_are_loaded() {
    crate::crafting::mod_kinds::tests::with_recipe(0, |recipes, _| {
        bare_env(|base| {
            let env = Env {
                recipes: &recipes,
                ..*base
            };
            let diamond = ItemStack::new("minecraft:diamond", 1);
            for kind in [Kind::Crafting, mod_crafting_table()] {
                let mut menu = Menu::new(kind);
                menu.set_slot(&env, 1, Some(ItemStack::new("test:input", 2)));
                assert!(menu.take_result_made());
                // A stack that is neither the recipe's nor marked as the mods' is not taken.
                menu.set_slot(&env, 0, Some(diamond.clone()));
                menu.click(&env, 0, 0, Input::Pickup).unwrap();
                assert!(menu.carried.is_none());
                menu.set_hook_result(&env, 0, Some(diamond.clone()));
                assert!(!menu.take_result_made(), "setting the result made nothing");
                menu.click(&env, 0, 0, Input::Pickup).unwrap();
                assert_eq!(menu.carried, Some(diamond.clone()));
                assert_ne!(menu.slot(1), Some(&ItemStack::new("test:input", 2)));
                assert!(menu.take_result_made());
                assert!(!menu.result_is_hooks());
            }
        });
    });
}
