import { useEffect, useMemo, useRef, useState } from "react";

import { ColorBendsBackground } from "../../components/ColorBendsBackground";
import type { I18n } from "../../i18n";
import {
  loadRememberedUsername,
  loadSystemCredential,
  persistLoginCredential,
} from "./credential-store";

import pantsFrame01 from "../../assets/images/pants/pants_01.webp";
import pantsFrame02 from "../../assets/images/pants/pants_02.webp";
import pantsFrame03 from "../../assets/images/pants/pants_03.webp";
import pantsFrame04 from "../../assets/images/pants/pants_04.webp";
import pantsFrame05 from "../../assets/images/pants/pants_05.webp";
import pantsFrame06 from "../../assets/images/pants/pants_06.webp";
import pantsFrame07 from "../../assets/images/pants/pants_07.webp";
import pantsFrame08 from "../../assets/images/pants/pants_08.webp";
import pantsFrame09 from "../../assets/images/pants/pants_09.webp";

const pantsFrames = [
  pantsFrame01,
  pantsFrame02,
  pantsFrame03,
  pantsFrame04,
  pantsFrame05,
  pantsFrame06,
  pantsFrame07,
  pantsFrame08,
  pantsFrame09,
];

const expressionModules = import.meta.glob<string>("../../assets/images/**/*.webp", {
  import: "default",
  query: "?url",
});

const expressionGroups = Object.entries(expressionModules).reduce(
  (groups, [path, load]) => {
    const parts = path.split("/");
    const groupName = parts[parts.length - 2];
    const fileName = parts[parts.length - 1] ?? "";
    if (!groupName || !/_[0-9]+\.webp$/i.test(fileName)) return groups;
    const frames = groups.get(groupName) ?? [];
    frames.push({ load, path });
    groups.set(groupName, frames);
    return groups;
  },
  new Map<string, Array<{ load: () => Promise<string>; path: string }>>(),
);

const expressions = Array.from(expressionGroups.entries()).map(
  ([name, frames]) => ({
    name,
    frames: frames
      .sort((left, right) => left.path.localeCompare(right.path))
      .map((frame) => frame.load),
  }),
);

export function PantsLogo() {
  const [frameIndex, setFrameIndex] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(
      () => setFrameIndex((current) => (current + 1) % pantsFrames.length),
      130,
    );
    return () => window.clearInterval(timer);
  }, []);
  return (
    <img
      alt="Aster"
      className="brand-mark"
      draggable={false}
      src={pantsFrames[frameIndex]}
    />
  );
}

export function LoginScreen({
  error,
  i18n,
  isLoginPending,
  notice,
  onOpenConnectionWizard,
  onOpenPasswordReset,
  onLogin,
}: {
  error: string | null;
  i18n: I18n;
  isLoginPending: boolean;
  notice: string | null;
  onOpenConnectionWizard: () => void;
  onOpenPasswordReset: () => void;
  onLogin: (username: string, password: string, remember: boolean) => Promise<void>;
}) {
  const rememberedUsername = useMemo(loadRememberedUsername, []);
  const [username, setUsername] = useState(rememberedUsername || "admin");
  const [password, setPassword] = useState("");
  const [remember, setRemember] = useState(Boolean(rememberedUsername));

  useEffect(() => {
    let active = true;
    if (rememberedUsername) {
      void loadSystemCredential(rememberedUsername).then((credential) => {
        if (active && credential) setPassword(credential.password);
      });
    }
    return () => {
      active = false;
    };
  }, [rememberedUsername]);

  return (
    <main className="login-shell">
      <section className="login-brand-panel">
        <ColorBendsBackground
          bandWidth={6}
          className="login-color-bends-bg"
          colors={["#ff5c7a", "#8a5cff", "#00ffd1"]}
          frequency={1}
          intensity={1.5}
          iterations={1}
          mouseInfluence={1}
          noise={0.15}
          parallax={0.5}
          rotation={90}
          scale={1}
          speed={0.2}
          transparent
          warpStrength={1}
        />
        <div className="login-brand-content">
          <ExpressionWall />
          <div className="login-copy">
            <h1>{i18n.t("login.title")}</h1>
            <p>{i18n.t("login.description")}</p>
          </div>
        </div>
      </section>
      <section className="login-card">
        <div className="login-card-main">
          <div className="login-card-header"><h2>{i18n.t("login.accountLogin")}</h2></div>
          {error ? <div className="error-banner login-message">{error}</div> : null}
          {notice ? <div className="notice-banner login-message">{notice}</div> : null}
          <form
            className="login-form"
            onSubmit={(event) => {
              event.preventDefault();
              void onLogin(username, password, remember);
            }}
          >
            <label><span>{i18n.t("login.username")}</span><input autoComplete="username" autoFocus disabled={isLoginPending} onChange={(event) => setUsername(event.target.value)} value={username} /></label>
            <label><span>{i18n.t("login.password")}</span><input autoComplete="current-password" disabled={isLoginPending} onChange={(event) => setPassword(event.target.value)} type="password" value={password} /></label>
            <div className="login-options-row">
              <label className="login-remember-check"><input checked={remember} disabled={isLoginPending} onChange={(event) => {
                setRemember(event.target.checked);
                if (!event.target.checked) void persistLoginCredential(username, password, false);
              }} type="checkbox" /><span>使用系统安全凭据记住密码</span></label>
              <button className="login-reset-toggle" disabled={isLoginPending} onClick={onOpenPasswordReset} type="button">{i18n.t("login.forgotPassword")}</button>
            </div>
            <button className="primary-button login-submit" disabled={isLoginPending} type="submit">{isLoginPending ? i18n.t("login.loggingIn") : i18n.t("login.login")}</button>
          </form>
          <div className="login-reset-panel"><button className="ghost-button login-connect-button" disabled={isLoginPending} onClick={onOpenConnectionWizard} type="button">连接主电脑</button></div>
        </div>
        <div className="login-support"><span>技术支持</span><strong>鲸天科技 · whalesky-labs · west · Liberty.</strong></div>
      </section>
    </main>
  );
}

function ExpressionWall() {
  const [frameIndex, setFrameIndex] = useState(0);
  useEffect(() => {
    const timer = window.setInterval(() => setFrameIndex((value) => value + 1), 130);
    return () => window.clearInterval(timer);
  }, []);
  return <div aria-hidden="true" className="login-expression-wall">{expressions.map((expression, index) => <LazyExpression expression={expression} frameIndex={frameIndex + index * 2} key={expression.name} />)}</div>;
}

function LazyExpression({
  expression,
  frameIndex,
}: {
  expression: (typeof expressions)[number];
  frameIndex: number;
}) {
  const [source, setSource] = useState<string | null>(null);
  const requestRef = useRef(0);
  useEffect(() => {
    const request = ++requestRef.current;
    const load = expression.frames[frameIndex % expression.frames.length];
    void load().then((nextSource) => {
      if (requestRef.current === request) setSource(nextSource);
    });
  }, [expression, frameIndex]);
  return (
    <div className="login-expression-item">
      {source ? <img alt="" draggable={false} src={source} /> : null}
    </div>
  );
}
