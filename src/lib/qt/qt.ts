// src/lib/qt.ts

type RpcPending = {
	resolve: (value: any) => void;
	reject: (reason?: any) => void;
};

type RpcResponse = {
	requestId: string;
	status: "ok" | "error";
	result?: any;
	error?: any;
};

type RpcRequest = {
	command: string;
	args: any;
	requestId: string;
};

type BackendEvent = {
	event: string;
	payload: any;
};

const isBrowser = typeof window !== "undefined";


/* ---------------------------
   Internal state
--------------------------- */

const rpcPending = new Map<string, RpcPending>();
const listeners = new Map<string, Set<(payload: any) => void>>();

let backend: any = null;

let readyPromise: Promise<void> | null = null;
let readyResolve!: () => void;

/* invoke queue */
type QueuedCall = () => void;
const invokeQueue: QueuedCall[] = [];

let initStarted = false;
let channelCreated = false;

/* ---------------------------
   Event system
--------------------------- */

function dispatch(message: string) {
	const msg: BackendEvent = JSON.parse(message);

	const set = listeners.get(msg.event);
	if (!set) return;

	for (const fn of set) {
		try {
			fn(msg);
		} catch (e) {
			console.error("[qt] event handler error:", e);
		}
	}
}

/* ---------------------------
   Safe backend fallback
--------------------------- */

const noopBackend = {
	invokeFromJs: () => console.warn("[qt] invoke ignored (no Qt context)"),
	eventFromJs: () => console.warn("[qt] emit ignored (no Qt context)"),
	eventEmitted: { connect: () => {} },
	invokeResponse: { connect: () => {} },
	init: () => {},
};

/* ---------------------------
   Qt bootstrap
--------------------------- */

function initQt(): Promise<void> {
	if (readyPromise) return readyPromise;
	if (initStarted) return readyPromise ?? Promise.resolve();

	initStarted = true;

	readyPromise = new Promise((resolve) => {
		readyResolve = resolve;

		const tryInit = () => {
			if (channelCreated) return;
			channelCreated = true;

			console.info("[qt] Initialising Qt Connection...");

			const qt = (window as any).qt;
			const QWebChannel = (window as any).QWebChannel;

			if (!qt || !QWebChannel) {
				console.error("[qt] missing qt or QWebChannel");
				return;
			}

			try {
				new QWebChannel(qt.webChannelTransport, (ch: any) => {
					backend = ch.objects.backend;

					if (!backend) {
						console.error("[qt] backend not found");
						resolve();
						return;
					}

					console.log("[qt] QWebChannel ready");

					backend.eventEmitted.connect((message: string) => {
						try {
							dispatch(message);
						} catch {
							console.error("[qt] invalid event payload");
						}
					});

					backend.invokeResponse.connect((payload: string) => {
						try {
							const msg: RpcResponse = JSON.parse(payload);

							const entry = rpcPending.get(msg.requestId);
							if (!entry) return;

							rpcPending.delete(msg.requestId);

							if (msg.status === "ok") entry.resolve(msg.result);
							else entry.reject(msg.error);
						} catch {
							console.error("[qt] invalid rpc response");
						}
					});

					backend.init?.();

					/* Flush the invoke queue */
					for (const fn of invokeQueue) fn();
					invokeQueue.length = 0;

					console.log("[qt] Initialisation Complete");

					resolve();
				});
			} catch (err) {
				console.error("[qt] Initialisation Failed:", err);
				backend = noopBackend;
				resolve();
			}
		};

		const timer = setInterval(() => {
			tryInit();

			if (channelCreated) {
				clearInterval(timer);
			}
		}, 25);
	});

	return readyPromise;
}

//
// Public API
//
export function ensureQtReady() {
	return initQt();
}

// Rust invoker, queue if we're not Qt connected
export async function invoke<T = any>(command: string, args: any = {}): Promise<T> {
	const requestId = crypto.randomUUID();

	return new Promise((resolve, reject) => {
		const exec = () => {
			if (!backend || backend === noopBackend) {
				reject(new Error("[qt] backend not ready"));
				return;
			}

			rpcPending.set(requestId, { resolve, reject });
			backend.invokeFromJs(JSON.stringify({ command, args, requestId } satisfies RpcRequest));
		};

		if (!backend || !channelCreated) {
			invokeQueue.push(exec);
			return;
		}

		exec();
	});
}

// Emit an Event to Rust
export async function emit(event: string, payload: any = {}) {
	if (!backend) return;

	backend.eventFromJs(JSON.stringify({ event, payload } satisfies BackendEvent));
}

// Listen to an Event from Rust
export function listen<T = any>(event: string, handler: (payload: T) => void): () => void {
	if (!listeners.has(event)) {
		listeners.set(event, new Set());
	}

	listeners.get(event)!.add(handler);

	return () => listeners.get(event)?.delete(handler);
}


// Listen to an Event from Rust once
export function once<T = any>(event: string, handler: (payload: T) => void): () => void {
	let off = () => {};

	const wrapper = (payload: T) => {
		off();
		handler(payload);
	};

	off = listen(event, wrapper);
	return off;
}
