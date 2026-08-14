import type { ReactNode } from "react";

export type IconName =
  | "home"
  | "inbox"
  | "projects"
  | "workspaces"
  | "apps"
  | "library"
  | "board"
  | "settings"
  | "search"
  | "capture"
  | "more"
  | "close"
  | "archive"
  | "trash"
  | "back";

const paths: Record<IconName, ReactNode> = {
  home: <><path d="M3.5 9 10 3.5 16.5 9"/><path d="M5.5 8v8h9V8"/></>,
  inbox: <><path d="M3.5 5.5h13v10h-13z"/><path d="M3.5 11h3l1.5 2h4l1.5-2h3"/></>,
  projects: <><path d="M3.5 5.5h5l1.5 2h6.5v8.5h-13z"/><path d="M3.5 7.5h13"/></>,
  workspaces: <><rect x="3.5" y="5" width="8.5" height="8.5"/><rect x="8" y="3.5" width="8.5" height="8.5"/><path d="M7 16.5h8.5v-8.5"/></>,
  apps: <><rect x="3.5" y="3.5" width="5" height="5"/><rect x="11.5" y="3.5" width="5" height="5"/><rect x="3.5" y="11.5" width="5" height="5"/><rect x="11.5" y="11.5" width="5" height="5"/></>,
  library: <><path d="M4 3.5h4.5v13H4zM8.5 3.5H13v13H8.5z"/><path d="m13 4.5 3-.7 1.5 11.7-3 .7z"/></>,
  board: <><rect x="3.5" y="4" width="4" height="12"/><rect x="9" y="4" width="3" height="8"/><rect x="13.5" y="4" width="3" height="10"/></>,
  settings: <><circle cx="10" cy="10" r="2.5"/><path d="M10 3.5v2M10 14.5v2M3.5 10h2M14.5 10h2M5.4 5.4l1.4 1.4M13.2 13.2l1.4 1.4M14.6 5.4l-1.4 1.4M6.8 13.2l-1.4 1.4"/></>,
  search: <><circle cx="8.5" cy="8.5" r="4.5"/><path d="m12 12 4 4"/></>,
  capture: <><path d="M10 3.5v13M3.5 10h13"/></>,
  more: <><circle cx="4.5" cy="10" r=".7"/><circle cx="10" cy="10" r=".7"/><circle cx="15.5" cy="10" r=".7"/></>,
  close: <path d="m4.5 4.5 11 11m0-11-11 11"/>,
  archive: <><path d="M3.5 6h13v10h-13zM2.5 3.5h15V6"/><path d="M8 9h4"/></>,
  trash: <><path d="M5.5 6.5h9l-.7 10h-7.6zM4 6.5h12M8 3.5h4l1 3"/></>,
  back: <><path d="m11.5 4.5-5.5 5.5 5.5 5.5"/><path d="M6 10h10"/></>,
};

export function Icon({ name, filled = false }: { name: IconName; filled?: boolean }) {
  return (
    <svg
      className="mos-icon"
      data-filled={filled || undefined}
      width="20"
      height="20"
      viewBox="0 0 20 20"
      aria-hidden="true"
      focusable="false"
    >
      {paths[name]}
    </svg>
  );
}
