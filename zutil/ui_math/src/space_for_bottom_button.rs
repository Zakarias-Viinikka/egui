// Space to reserve at the bottom of a page for a full-width button,
// plus the separator and vertical spacing around it.

const BUTTON_HEIGHT: f32 = 40.0;
const SEPARATOR_AND_SPACING: f32 = 20.0;

pub fn space_for_bottom_button() -> f32 {
    BUTTON_HEIGHT + SEPARATOR_AND_SPACING
}

pub fn content_height_above_bottom_button(available_height: f32) -> f32 {
    (available_height - space_for_bottom_button()).max(0.0)
}
