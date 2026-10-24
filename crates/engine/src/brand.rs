//! The app's name as teachers see it.

/// The display name.
pub const APP_NAME: &str = "EduDeck";

/// The credit line in the About box.
pub const BASED_ON: &str = "Based on DeckCraft by the ArtCraft Team";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_is_edudeck_and_deckcraft_is_credited() {
        assert_eq!(APP_NAME, "EduDeck");
        assert!(BASED_ON.contains("DeckCraft"));
    }
}
