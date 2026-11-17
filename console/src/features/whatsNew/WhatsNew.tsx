import { Sparkles } from "lucide-react";
import { Fragment, type ReactNode, useState } from "react";
import { Link, Tooltip, TooltipTrigger } from "react-aria-components";
import { navigation } from "@/app/navigation";
import { Alert } from "@/components/Alert";
import { Button } from "@/components/Button";
import { Dialog } from "@/components/Dialog";
import { EmptyState } from "@/components/EmptyState";
import {
  dateRange,
  type Entry,
  type Group,
  groupEntries,
  groupTitle,
  IMPACT_HEADINGS,
  TYPE_LABELS,
} from "./changelog";
import { changelog as bundled, type Changelog } from "./entries";
import {
  isUnread,
  markerFor,
  type SeenMarker,
  seenOnArrival,
  unreadCount,
  writeSeen,
} from "./seen";
import styles from "./WhatsNew.module.css";

/** "What's new: 3 new changes", the button's name and tooltip; "What's new" with none. */
export function whatsNewLabel(unread: number): string {
  if (unread === 0) return "What's new";
  return `What's new: ${unread} new ${unread === 1 ? "change" : "changes"}`;
}

/**
 * The top bar's What's new button and the list it opens. The button counts the changes newer
 * than the newest one this console has shown on this computer; on the first visit the newest
 * change counts as seen at once. Opening the list clears the count, and the list marks those
 * changes New until it closes.
 */
export function WhatsNew({ changelog = bundled }: { changelog?: Changelog }) {
  const [seen, setSeen] = useState(() => seenOnArrival(changelog.entries));
  // What was seen before this opening, so the open list can mark what is new.
  const [shownSince, setShownSince] = useState<SeenMarker | null | undefined>(undefined);
  const unread = unreadCount(changelog.entries, seen);
  const label = whatsNewLabel(unread);
  function open() {
    setShownSince(seen);
    const marker = markerFor(changelog.entries);
    if (marker !== null) {
      writeSeen(marker);
      setSeen(marker);
    }
  }
  return (
    <>
      <TooltipTrigger isDisabled={unread === 0}>
        <Button variant="quiet" aria-label={label} onPress={open}>
          <Sparkles aria-hidden="true" />
          What's new
          {unread > 0 && (
            <span aria-hidden="true" className={styles.count}>
              {unread}
            </span>
          )}
        </Button>
        <Tooltip placement="bottom" className={styles.tooltip}>
          {label}
        </Tooltip>
      </TooltipTrigger>
      <Dialog
        title="What's new"
        description="Changes to the engine and the console, newest release first, most important first."
        isOpen={shownSince !== undefined}
        onOpenChange={(isOpen) => !isOpen && setShownSince(undefined)}
        size="wide"
        actions={<Button onPress={() => setShownSince(undefined)}>Close</Button>}
      >
        <ChangeList
          changelog={changelog}
          seen={shownSince ?? null}
          onNavigate={() => setShownSince(undefined)}
        />
      </Dialog>
    </>
  );
}

function ChangeList({
  changelog,
  seen,
  onNavigate,
}: {
  changelog: Changelog;
  seen: SeenMarker | null;
  onNavigate: () => void;
}) {
  if (changelog.error !== null) {
    return (
      <Alert tone="danger" title="The list of changes could not be read">
        <p>{changelog.error}</p>
      </Alert>
    );
  }
  if (changelog.entries.length === 0) {
    return (
      <EmptyState
        title="No changes recorded yet"
        description="Changes to the engine and the console are listed here as they are made."
      />
    );
  }
  return (
    <div className={styles.list}>
      {groupEntries(changelog.entries).map((group) => (
        <GroupView key={groupTitle(group)} group={group} seen={seen} onNavigate={onNavigate} />
      ))}
    </div>
  );
}

function GroupView({
  group,
  seen,
  onNavigate,
}: {
  group: Group;
  seen: SeenMarker | null;
  onNavigate: () => void;
}) {
  const title = groupTitle(group);
  return (
    <section aria-label={`${title}, ${dateRange(group)}`} className={styles.group}>
      <h3 className={styles.groupTitle}>
        {title} <span className={styles.range}>{dateRange(group)}</span>
      </h3>
      {group.sections.map((section) => (
        <Fragment key={section.impact}>
          <h4 className={styles.impact}>{IMPACT_HEADINGS[section.impact]}</h4>
          <ul className={styles.entries}>
            {section.entries.map((entry) => (
              <EntryView
                key={entry.file}
                entry={entry}
                isNew={isUnread(entry, seen)}
                onNavigate={onNavigate}
              />
            ))}
          </ul>
        </Fragment>
      ))}
    </section>
  );
}

function EntryView({
  entry,
  isNew,
  onNavigate,
}: {
  entry: Entry;
  isNew: boolean;
  onNavigate: () => void;
}) {
  const destination = entry.link === undefined ? undefined : screenLabel(entry.link);
  return (
    <li className={styles.entry}>
      <h5 className={styles.title}>
        <span className={styles.type}>{TYPE_LABELS[entry.type]}</span> {entry.title}
        {isNew && (
          <>
            {" "}
            <span className={styles.new}>New</span>
          </>
        )}
      </h5>
      <p className={styles.meta}>
        <time dateTime={entry.date}>{entry.date}</time>
        {entry.area && ` · ${entry.area}`}
      </p>
      {paragraphs(entry.summary).map((text) => (
        <p key={text}>{inlineCode(text)}</p>
      ))}
      {entry.link !== undefined && (
        <Link href={entry.link} className={styles.link} onPress={onNavigate}>
          Open {destination}
        </Link>
      )}
    </li>
  );
}

/** The navigation label of the screen a console route belongs to, such as Settings. */
export function screenLabel(link: string): string {
  const screen = navigation
    .flatMap((section) => section.screens)
    .filter((each) => link === each.path || link.startsWith(`${each.path}/`))
    .sort((a, b) => b.path.length - a.path.length)[0];
  return screen?.label ?? "the page";
}

function paragraphs(text: string): string[] {
  return text
    .split(/\n\s*\n/)
    .map((paragraph) => paragraph.replace(/\s*\n\s*/g, " ").trim())
    .filter((paragraph) => paragraph !== "");
}

/** Summaries are plain text; `backticks` mark a command or a setting. */
function inlineCode(text: string): ReactNode[] {
  return text.split(/(`[^`]+`)/).map((part, index) =>
    part.startsWith("`") && part.endsWith("`") && part.length > 1 ? (
      // biome-ignore lint/suspicious/noArrayIndexKey: the parts of one fixed string never reorder.
      <code key={index}>{part.slice(1, -1)}</code>
    ) : (
      part
    ),
  );
}
