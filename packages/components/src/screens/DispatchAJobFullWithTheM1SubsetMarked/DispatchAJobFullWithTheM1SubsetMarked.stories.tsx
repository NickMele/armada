import type { Meta, StoryObj } from "@storybook/react-vite";
import { Absent } from "../absent";

/**
 * Journey · Dispatch a Job. The full design and M1's reduced version of it,
 * side by side, with the two things that are designed and not built at M1
 * dimmed either side.
 *
 * **The two bright columns are `Approval card` and `Job composer`, and neither
 * is built.** The drawing names both as bare `data-component` values. Their
 * fields are all primitives that exist — Input, Textarea, Select, Button — but
 * the card that arranges them is the thing being agreed, so the region is named
 * rather than assembled on the spot.
 */
const meta: Meta = {
  title: "Screens/Dispatch a job — full, with the M1 subset marked",
};
export default meta;

type Story = StoryObj;

export const Dispatch: Story = {
  render: () => (
    <div className="armada-screen">
      <div className="armada-screen__legend">
        <div className="armada-screen__legend-line">
          <span className="armada-screen__swatch" />
          <span className="armada-screen__strong">M1 renders this</span>
        </div>
        <div className="armada-screen__legend-line">
          <span className="armada-screen__swatch" data-dim />
          <span>
            Designed, not built at M1 — dimmed to{" "}
            <span className="armada-screen__mono">--border-subtle</span> and{" "}
            <span className="armada-screen__mono">--fg-subtle</span>, the same dimming a
            de-emphasised row takes
          </span>
        </div>
      </div>

      <div className="armada-screen__row">
        {/* 1 — the Job Board queue, dimmed. M1 has no queue to browse. */}
        <div className="armada-screen__card" data-dim data-width="narrow">
          <div className="armada-screen__card-head">
            <span className="armada-screen__caption">1. Browse the Job Board</span>
            <span className="armada-screen__tag" data-dim>
              not at M1
            </span>
          </div>
          <div className="armada-screen__queue">
            <div className="armada-screen__queue-head">
              <span>Ready</span>
              <span className="armada-screen__mono">3</span>
            </div>
            <div className="armada-screen__queue-item">
              <span>Coalesce concurrent token refreshes</span>
              <span>api/auth · found by Fleet</span>
            </div>
            <div className="armada-screen__queue-item">
              <span>Retire the legacy poke path</span>
              <span>core/fleet · drafted in Helm</span>
            </div>
            <div className="armada-screen__queue-item">
              <span>Cache the manifest read</span>
              <span>core/manifest · dispatched by you</span>
            </div>
          </div>
          <span className="armada-screen__caption">
            The queue, its origin tags, the list-and-graph toggle and the scope picker. M1
            has no queue: a job is created and immediately waiting on you, so there is
            nothing to browse.
          </span>
        </div>

        {/* 2 — the approval card, full. Not built. */}
        <div className="armada-screen__card" data-bright data-width="card">
          <div className="armada-screen__card-head">
            <span className="armada-screen__caption">
              2. The approval card — the full design
            </span>
            <span className="armada-screen__tag">reduced at M1</span>
          </div>
          <div className="armada-screen__slot">
            <Absent
              name="Approval card"
              note={
                "Holds the title “Coalesce concurrent token refreshes”, its brief, then " +
                "the three glance fields the card exists for — Diff size ~4 files, Job " +
                "type feature, Cost, estimated ~$3.20 of $20 — then Workflow bug, 4 " +
                "steps · Workspace armada · Criteria 4, then Cancel beside Approve and " +
                "dispatch."
              }
            />
          </div>
          <span className="armada-screen__caption" data-muted>
            The three glance fields are the whole point of the card: diff size, job type and
            estimated cost have to be read before the tap registers. Criteria is the one row
            M1 drops, because there is no Judge to hold them.
          </span>
        </div>

        {/* 3 — what M1 renders. Also not built. */}
        <div className="armada-screen__card" data-bright data-width="card">
          <div className="armada-screen__card-head">
            <span className="armada-screen__caption">3. What M1 renders</span>
            <span className="armada-screen__tag">M1</span>
          </div>
          <div className="armada-screen__slot">
            <Absent
              name="Job composer"
              note={
                "Holds a Title input, a Brief textarea, a Workflow select reading “bug — " +
                "4 steps” beside a read-only Project armada, then the two-up glance strip " +
                "Steps 4 · 2 gated and Checks build, test, then Cancel beside Approve and " +
                "dispatch — the one accent fill in the whole milestone."
              }
            />
          </div>
          <span className="armada-screen__caption" data-muted>
            <span className="armada-screen__strong">
              Approve lands the job in <span className="armada-screen__mono">queued</span>,
              not <span className="armada-screen__mono">running</span>.
            </span>{" "}
            A drone spawning is what starts it, and at M1 Fleet runs one at a time, so a job
            approved while another is working sits queued for as long as that one takes. Its
            badge carries <span className="armada-screen__mono">cpu</span> rather than{" "}
            <span className="armada-screen__mono">clock</span>, because a reason&rsquo;s
            glyph replaces <span className="armada-screen__mono">clock</span> where one is
            present and M1&rsquo;s only reason is{" "}
            <span className="armada-screen__mono">waiting_on_resources</span> — there are no
            dependencies to be blocked by. Same card, same order, same button, one field set
            smaller. The glance strip survives with the two values M1 can measure before
            dispatch — how long the workflow is and which Checks gate it — because a card
            whose whole design is a forced glance cannot ship with nothing to glance at.{" "}
            <span className="armada-screen__strong">
              Cancel writes <span className="armada-screen__mono">killed</span>.
            </span>{" "}
            A job you never dispatched was not stopped, it was abandoned, so the copy names
            what the person is doing while the record names what happened.{" "}
            <span className="armada-screen__mono">rejected</span> is the verdict exit and it
            is out of M1, so <span className="armada-screen__mono">killed</span> is the only
            destination — an operator act carrying no verdict, which is the honest reading of
            closing a card.
          </span>
        </div>

        {/* Forks off the main line, dimmed. */}
        <div className="armada-screen__card" data-dim data-width="narrow">
          <div className="armada-screen__card-head">
            <span className="armada-screen__caption">Forks off the main line</span>
            <span className="armada-screen__tag" data-dim>
              not at M1
            </span>
          </div>
          <div className="armada-screen__stack">
            <div className="armada-screen__fork">
              <span>Pre-approved before you step away</span>
              <span>
                Specific queued jobs marked to dispatch in your absence, indefinite until run
                or revoked.
              </span>
            </div>
            <div className="armada-screen__fork">
              <span>Pattern learning</span>
              <span>
                After the same command trips the allowlist N times, Armada proposes a
                Manifest change. You confirm or decline.
              </span>
            </div>
            <div className="armada-screen__fork">
              <span>Criteria editor</span>
              <span>
                The only place acceptance criteria are authored. Nothing reads them until the
                Judge exists.
              </span>
            </div>
          </div>
          <span className="armada-screen__caption">
            All three sit outside the card and none is on the path through it, which is why
            the reduced version is a subset rather than a redraw.
          </span>
        </div>
      </div>

      <p className="armada-screen__note">
        <span className="armada-screen__strong">
          Approval stays one-by-one, and the accent is spent here.
        </span>{" "}
        Approve and dispatch is the only accent fill in the whole milestone: it is the single
        primary action M1 has, and every other screen&rsquo;s controls are secondary or
        ghost. The keyboard path is the same shape as the mouse path — the form is a tab
        order ending on the primary, and Enter from any field commits it.
      </p>
    </div>
  ),
};
