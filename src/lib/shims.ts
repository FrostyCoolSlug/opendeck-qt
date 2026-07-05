import { invoke } from "../lib/qt/qt.ts";

globalThis.open = (url?: string | URL) => {
	if (url) invoke("open_url", { url });
	return null;
};
