//! The `prepare_smithing_result` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// A smithing table's result was made for a server player at `pos`, and may be changed or
    /// cleared (Paper's `PrepareSmithingEvent` with `setResult`). `inputs` is the template, the
    /// base and the addition (`None` for an empty slot). `result` arrives holding the recipe's
    /// result, or what `prepare_smithing` set where no recipe matched, or `None`; each listener
    /// sees what the one before it left. Set another stack, or `None` to leave the result slot
    /// empty. Heard after `prepare_smithing`, when a click changed the inputs. Guest state is
    /// not swapped in. Server-side.
    #[hook(
        side = "server",
        contract = "listen",
        neoforge = "none (NeoForge has no such event)",
        paper = "PrepareSmithingEvent"
    )]
    fn prepare_smithing_result();
}
