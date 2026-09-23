import { GUIDE_GROUPS, GUIDES } from "../../guides";
import type { Guide } from "../../guides/guide";

/**
 * Every guide, numbered and grouped, readable end to end.
 *
 * **Drawn whole rather than as a list of titles.** A person who wants to
 * understand how jobs work reads this from the top, and a rail of titles with
 * one entry open beside it is a surface you can only read one card at a time —
 * which is the card they already have.
 *
 * **Nothing here is selected, opened or filtered.** The catalogue is a
 * document. What it costs is a long page; what it buys is that reading it
 * needs no controls and no state to get wrong.
 */
export type GuideCatalogueProps = {
  /** Every guide, in catalogue order. The real set by default. */
  guides?: readonly Guide[];
};

export function GuideCatalogue({ guides = GUIDES }: GuideCatalogueProps) {
  const groups = GUIDE_GROUPS.map((group) => ({
    group,
    entries: guides.filter((one) => one.group === group.id),
  })).filter((section) => section.entries.length > 0);

  return (
    <div className="armada-guides" aria-label="Guides">
      <div className="armada-guides__head">
        <h1 className="armada-guides__heading">Guides</h1>
        <p className="armada-guides__says">
          {guides.length} guides. A <span className="mono">?</span> beside a piece opens its guide
          where you are.
        </p>
      </div>

      {groups.map(({ group, entries }) => (
        <section key={group.id} className="armada-guides__group" aria-label={group.title}>
          <h2 className="armada-guides__group-title">{group.title}</h2>
          <ol className="armada-guides__entries">
            {entries.map((guide) => (
              <Entry key={guide.number} guide={guide} />
            ))}
          </ol>
        </section>
      ))}
    </div>
  );
}

/** One entry, drawn the way the card draws it: number, title, picture, prose. */
function Entry({ guide }: { guide: Guide }) {
  return (
    <li className="armada-guides__entry">
      <span className="armada-guides__number mono">{guide.number}</span>
      <div className="armada-guides__entry-body">
        <h3 className="armada-guides__title">{guide.title}</h3>
        {guide.picture === undefined ? null : (
          <img className="armada-guides__picture" src={guide.picture.src} alt={guide.picture.alt} />
        )}
        {guide.body.map((paragraph) => (
          <p key={paragraph} className="armada-guides__paragraph">
            {paragraph}
          </p>
        ))}
      </div>
    </li>
  );
}
