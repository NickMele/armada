import type { Meta, StoryObj } from "@storybook/react-vite";

import { Provenance } from "./Provenance";

/** One proposal line's source, in each of the five the wire names. */
const meta: Meta<typeof Provenance> = {
  title: "Compositions/Provenance",
  component: Provenance,
};
export default meta;

type Story = StoryObj<typeof Provenance>;

/** A port a compose file declares: the file, in mono, and nothing else. */
export const Read: Story = { args: { source: "read", file: "docker-compose.yml", at: "services.db.ports" } };

/** A script placed by guess: the file it was read from, and the word that says the placement is a guess. */
export const Convention: Story = { args: { source: "convention", file: "package.json", at: "scripts.test" } };

/** A policy row: nothing in a repository corresponds to it. */
export const Default: Story = { args: { source: "default" } };

export const EditedDuringSetup: Story = { name: "Edited during setup", args: { source: "edited_during_setup" } };

export const AddedDuringSetup: Story = { name: "Added during setup", args: { source: "added_during_setup" } };
