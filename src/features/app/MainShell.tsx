import {
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { NavKey } from "../../entities/navigation";
import type { AppStatus } from "../../entities/runtime";
import type { I18n } from "../../i18n";
import { PantsLogo } from "../auth/LoginScreen";
import { GitHubIcon, NavIcon, navGroups, type NavItem } from "../navigation/navigation";

const SIDEBAR_MARQUEE_SPEED_PX_PER_SECOND = 28;
const SIDEBAR_MARQUEE_MIN_DURATION_SECONDS = 8;

function SidebarConnectionHint({ text }: { text: string }) {
  const viewportRef = useRef<HTMLElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);
  const [marquee, setMarquee] = useState({
    durationSeconds: SIDEBAR_MARQUEE_MIN_DURATION_SECONDS,
    isOverflowing: false,
  });

  useLayoutEffect(() => {
    const viewport = viewportRef.current;
    const textElement = textRef.current;
    if (!viewport || !textElement) return;

    const updateMarquee = () => {
      const textWidth = textElement.scrollWidth;
      const isOverflowing = textWidth > viewport.clientWidth + 1;
      const durationSeconds = Math.max(
        SIDEBAR_MARQUEE_MIN_DURATION_SECONDS,
        textWidth / SIDEBAR_MARQUEE_SPEED_PX_PER_SECOND,
      );

      setMarquee((current) => {
        if (
          current.isOverflowing === isOverflowing &&
          current.durationSeconds === durationSeconds
        ) {
          return current;
        }
        return { durationSeconds, isOverflowing };
      });
    };

    updateMarquee();
    if (typeof ResizeObserver === "undefined") {
      window.addEventListener("resize", updateMarquee);
      return () => window.removeEventListener("resize", updateMarquee);
    }

    const resizeObserver = new ResizeObserver(updateMarquee);
    resizeObserver.observe(viewport);
    resizeObserver.observe(textElement);
    return () => resizeObserver.disconnect();
  }, [text]);

  return (
    <em
      className="sidebar-connection-marquee"
      data-overflowing={marquee.isOverflowing}
      ref={viewportRef}
      style={{
        "--sidebar-connection-marquee-duration": `${marquee.durationSeconds}s`,
      } as CSSProperties}
    >
      <span className="sidebar-connection-marquee-track">
        <span className="sidebar-connection-marquee-item" ref={textRef}>{text}</span>
        {marquee.isOverflowing ? (
          <span aria-hidden="true" className="sidebar-connection-marquee-item">{text}</span>
        ) : null}
      </span>
    </em>
  );
}

export function MainShell({
  activeNav,
  children,
  connectionHint,
  connectionKind,
  connectionLabel,
  desktopPlatform,
  footerStatus,
  i18n,
  logout,
  refresh,
  setActiveNav,
  settingsNavItem,
  status,
  visibleNavItems,
}: {
  activeNav: NavKey;
  children: ReactNode;
  connectionHint: string;
  connectionKind: string;
  connectionLabel: string;
  desktopPlatform: string;
  footerStatus: { kind: string; text: string };
  i18n: I18n;
  logout: () => Promise<void>;
  refresh: () => Promise<void>;
  setActiveNav: (key: NavKey) => void;
  settingsNavItem?: NavItem;
  status: AppStatus | null;
  visibleNavItems: NavItem[];
}) {
  return (
    <div className="app-shell" data-platform={desktopPlatform}>
      <aside className="sidebar">
        <div className="brand">
          <PantsLogo />
          <div><strong>Aster</strong><span>{i18n.t("app.productTagline")}</span></div>
        </div>
        <nav className="nav-list">
          {navGroups.map((group) => {
            const groupItems = group.keys
              .map((key) => visibleNavItems.find((item) => item.key === key))
              .filter((item): item is NavItem => Boolean(item));
            if (groupItems.length === 0) return null;
            return (
              <div className="nav-section" key={group.titleKey}>
                <span className="nav-section-title">{i18n.t(group.titleKey)}</span>
                {groupItems.map((item) => (
                  <button
                    className={activeNav === item.key ? "nav-item active" : "nav-item"}
                    key={item.key}
                    onClick={() => setActiveNav(item.key)}
                  >
                    <NavIcon name={item.key} />
                    <span className="nav-item-label">{i18n.t(item.labelKey)}</span>
                  </button>
                ))}
              </div>
            );
          })}
        </nav>
        {settingsNavItem ? (
          <div className="sidebar-footer">
            <button
              className={activeNav === settingsNavItem.key ? "nav-item active" : "nav-item"}
              onClick={() => setActiveNav(settingsNavItem.key)}
            >
              <NavIcon name={settingsNavItem.key} />
              <span className="nav-item-label">{i18n.t(settingsNavItem.labelKey)}</span>
            </button>
            <button
              className={`sidebar-connection sidebar-connection-${connectionKind}`}
              onClick={() => setActiveNav(settingsNavItem.key)}
              title={connectionHint}
            >
              <span className="sidebar-connection-dot" />
              <span className="sidebar-connection-copy">
                <strong>{connectionLabel}</strong>
                <SidebarConnectionHint text={connectionHint} />
              </span>
            </button>
          </div>
        ) : null}
      </aside>
      <main className="content">
        <header className="topbar">
          <h1>{i18n.t(visibleNavItems.find((item) => item.key === activeNav)?.labelKey ?? "app.home")}</h1>
          <div className="topbar-actions">
            <button aria-label={i18n.t("app.githubAria")} className="ghost-button icon-button" onClick={() => void openUrl("https://github.com/westng")} title="GitHub"><GitHubIcon /></button>
            <button className="ghost-button" onClick={() => void refresh()}>{i18n.t("app.refreshStatus")}</button>
            <button className="ghost-button" onClick={() => void logout()}>{i18n.t("app.logout")}</button>
          </div>
        </header>
        <div className="content-body">{children}</div>
        <footer className={`app-statusbar app-statusbar-${footerStatus.kind}`}>
          <span className="app-statusbar-indicator" />
          <span className="app-statusbar-message">{footerStatus.text}</span>
          <span className="app-statusbar-meta">
            {status ? `${i18n.modeLabel(status.runtime.mode)} · Schema v${status.schemaVersion}` : i18n.t("app.initializing")}
          </span>
        </footer>
      </main>
    </div>
  );
}
