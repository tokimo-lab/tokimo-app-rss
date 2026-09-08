import {
  type AppRuntimeCtx,
  type Dispose,
  defineApp,
  RuntimeProvider,
} from "@tokimo/sdk";
import { ConfigProvider, ToastProvider, enUS as uiEnUS, zhCN as uiZhCN } from "@tokimo/ui";
import { StrictMode, type ReactNode, useEffect, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { enUS, zhCN } from "./i18n";
import { RssApp } from "./RssApp";
import "./index.css";

function UiProviders({ ctx, children }: { ctx: AppRuntimeCtx; children: ReactNode }) {
  const [locale, setLocale] = useState(ctx.locale);
  useEffect(() => ctx.shell.subscribeLocale(setLocale), [ctx.shell]);
  const uiLocale = locale.startsWith("zh") ? uiZhCN : uiEnUS;
  return (
    <ConfigProvider locale={uiLocale} dateFormat={{}}>
      <ToastProvider>{children}</ToastProvider>
    </ConfigProvider>
  );
}

export default defineApp({
  id: "rss",
  manifest: {
    id: "rss",
    appName: "RSS Subscriptions",
    icon: "Rss",
    image: "icon.png",
    color: "#f97316",
    windowType: "rss",
    defaultSize: { width: 1180, height: 760 },
    category: "app",
  },
  translations: { "zh-CN": zhCN, "en-US": enUS },
  mount(container, ctx): Dispose {
    const root: Root = createRoot(container);
    root.render(
      <StrictMode>
        <RuntimeProvider value={ctx}>
          <UiProviders ctx={ctx}>
            <RssApp ctx={ctx} />
          </UiProviders>
        </RuntimeProvider>
      </StrictMode>,
    );
    return () => root.unmount();
  },
});
