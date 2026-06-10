/**
 * Minimal /etc/hosts parser.
 *
 * The file is modelled as an ordered list of lines so that comments, blank
 * lines and section headers are preserved on save. Only `entry` lines are
 * shown in the structured editor; `raw` lines round-trip untouched.
 */

export interface HostEntry {
  type: "entry";
  id: string;
  enabled: boolean;
  ip: string;
  hosts: string; // space-separated hostnames
  comment: string;
}

export interface RawLine {
  type: "raw";
  id: string;
  text: string;
}

export type HostLine = HostEntry | RawLine;

let uid = 0;
function nextId() {
  uid += 1;
  return `h${uid}`;
}

const IP_RE = /^(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}|[0-9a-fA-F:]*:[0-9a-fA-F:]+)$/;

function looksLikeEntry(s: string): boolean {
  const parts = s.trim().split(/\s+/);
  return parts.length >= 2 && IP_RE.test(parts[0]);
}

function parseEntryParts(s: string): { ip: string; hosts: string; comment: string } {
  const hashIdx = s.indexOf("#");
  let main = s;
  let comment = "";
  if (hashIdx >= 0) {
    comment = s.slice(hashIdx + 1).trim();
    main = s.slice(0, hashIdx);
  }
  const parts = main.trim().split(/\s+/);
  return {
    ip: parts[0] ?? "",
    hosts: parts.slice(1).join(" "),
    comment,
  };
}

export function parseHosts(text: string): HostLine[] {
  return text.split("\n").map((line): HostLine => {
    const trimmed = line.trim();

    if (trimmed.startsWith("#")) {
      const uncommented = trimmed.replace(/^#+\s*/, "");
      if (looksLikeEntry(uncommented)) {
        const { ip, hosts, comment } = parseEntryParts(uncommented);
        return { type: "entry", id: nextId(), enabled: false, ip, hosts, comment };
      }
      return { type: "raw", id: nextId(), text: line };
    }

    if (trimmed !== "" && looksLikeEntry(trimmed)) {
      const { ip, hosts, comment } = parseEntryParts(trimmed);
      return { type: "entry", id: nextId(), enabled: true, ip, hosts, comment };
    }

    return { type: "raw", id: nextId(), text: line };
  });
}

export function serializeHosts(lines: HostLine[]): string {
  return lines
    .map((l) => {
      if (l.type === "raw") return l.text;
      const body = `${l.ip}\t${l.hosts}`.trimEnd();
      const withComment = l.comment ? `${body}  # ${l.comment}` : body;
      return l.enabled ? withComment : `# ${withComment}`;
    })
    .join("\n");
}

export function newEntry(): HostEntry {
  return { type: "entry", id: nextId(), enabled: true, ip: "", hosts: "", comment: "" };
}
