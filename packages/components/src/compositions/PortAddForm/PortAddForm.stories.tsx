import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { PortAddForm } from "./PortAddForm";

/**
 * Adding a port, as both the proposal sheet and the Manifest forms mount it. The variable
 * is derived from the name until a person types their own.
 */
const meta: Meta<typeof PortAddForm> = {
  title: "Compositions/Port add form",
  component: PortAddForm,
};
export default meta;

type Story = StoryObj<typeof PortAddForm>;

/** Name and port asked, the variable filled in, and typing over it stops the derivation. */
export const Empty: Story = {
  args: { taken: [], onAdd: fn() },
  play: async ({ args, canvasElement, userEvent }) => {
    const form = within(canvasElement);
    await expect(form.getByRole("button", { name: "Add a port" })).toBeDisabled();
    await userEvent.type(form.getByLabelText("Service name"), "web");
    await expect(form.getByLabelText("Variable")).toHaveValue("WEB_PORT");
    await userEvent.type(form.getByLabelText("Container port"), "3000");
    await userEvent.clear(form.getByLabelText("Variable"));
    await userEvent.type(form.getByLabelText("Variable"), "PORT");
    await userEvent.type(form.getByLabelText("Service name"), "site");
    await expect(form.getByLabelText("Variable")).toHaveValue("PORT");
    await userEvent.click(form.getByRole("button", { name: "Add a port" }));
    await expect(args.onAdd).toHaveBeenCalledWith({ name: "website", container: "3000", env: "PORT" });
  },
};

/** A variable another workspace maps warns in the field, and Add stays offered. */
export const SharedVariable: Story = {
  args: { taken: [], elsewhere: [{ env: "API_PORT", dir: "services/api" }], onAdd: fn() },
  play: async ({ canvasElement, userEvent }) => {
    const form = within(canvasElement);
    await userEvent.type(form.getByLabelText("Service name"), "api");
    await userEvent.type(form.getByLabelText("Container port"), "8080");
    await expect(form.getByText(/services\/api also maps a port to API_PORT/)).toBeVisible();
    await expect(form.getByRole("button", { name: "Add a port" })).toBeEnabled();
  },
};

/** A name already declared is refused where it is typed. */
export const TakenName: Story = {
  args: { taken: ["db"], onAdd: fn() },
  play: async ({ canvasElement, userEvent }) => {
    const form = within(canvasElement);
    await userEvent.type(form.getByLabelText("Service name"), "db");
    await userEvent.type(form.getByLabelText("Container port"), "5432");
    await expect(form.getByText("A port named db is already declared here.")).toBeVisible();
    await expect(form.getByRole("button", { name: "Add a port" })).toBeDisabled();
  },
};
