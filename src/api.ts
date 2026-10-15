import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { KlineData, Settings, StateSnapshot, StockRef } from "./types";

export const api = {
  getState: () => invoke<StateSnapshot>("get_state"),
  loginThs: (account: string, passwd: string) =>
    invoke<StateSnapshot>("login_ths", { account, passwd }),
  importCookie: (cookieText: string) =>
    invoke<StateSnapshot>("import_cookie", { cookieText }),
  logoutThs: () => invoke<void>("logout_ths"),
  syncNow: () => invoke<StateSnapshot>("sync_now"),
  addLocalStock: (input: string) => invoke<StockRef>("add_local_stock", { input }),
  removeLocalStock: (market: string, code: string) =>
    invoke<void>("remove_local_stock", { market, code }),
  getKline: (symbol: string, period: "day" | "minute") =>
    invoke<KlineData>("get_kline", { symbol, period }),
  setSettings: (settings: Settings) => invoke<void>("set_settings", { settings }),
  openThsLogin: () => invoke<void>("open_ths_login"),
  confirmThsLogin: () => invoke<StateSnapshot>("confirm_ths_login"),
  openThsForgot: () => invoke<void>("open_ths_forgot"),
};

export function onQuotesUpdate(cb: (quotes: import("./types").Quote[]) => void): Promise<UnlistenFn> {
  return listen<import("./types").Quote[]>("quotes-update", (e) => cb(e.payload));
}

export function onGroupsUpdate(cb: () => void): Promise<UnlistenFn> {
  return listen("groups-update", () => cb());
}

export function onAuthExpired(cb: (msg: string) => void): Promise<UnlistenFn> {
  return listen<string>("ths-auth-expired", (e) => cb(e.payload));
}
