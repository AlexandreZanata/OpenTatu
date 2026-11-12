# Recipe: add a reusable UI component

Only when no existing component fits ([../COMPONENTS.md](../COMPONENTS.md)) and the markup
is (or will be) used in more than one place.

1. Add a macro to `crates/dashboard/templates/components/ui.html`, with a `{#- ... -#}`
   comment above it stating its parameters (English):

   ```text
   {#- status_pill(label, modifier): small rounded state label. -#}
   {% macro status_pill(label, modifier) %}<span class="c-pill c-pill--{{ modifier }}">{{ label }}</span>{% endmacro %}
   ```

   Components with inner content print `{{ caller() }}` and are called with
   `{% call ui::name(args) %} ... {% endcall %}`.
2. If the component needs computed data (labels, CSS modifiers from domain values), add a
   struct/function in `crates/dashboard/src/view.rs` — the only place that maps domain values
   to UI text — with a test.
3. Add CSS in `crates/dashboard/static/app.css` as a `c-<name>` block using only the tokens in
   `:root` (add a token if needed). No inline styles.
4. Use it in at least one page and cover it in a router test.
5. Add it to the UI table in [../COMPONENTS.md](../COMPONENTS.md) and to
   `crates/dashboard/AGENTS.md`.
6. If an existing page has similar markup, replace it with the component in the same change.
7. `cargo xtask check`.
