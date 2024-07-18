var f = {
	fetch: (...[{ headers: o }, n]) => {
		const a = o.get("Upgrade");
		if (!a || a !== "websocket")
			return new Response("Expected Upgrade: WebSocket", { status: 426 });
		const t = new p();
		return (
			t[1] &&
				(t[1].accept(),
				t[1].addEventListener("message", async ({ data: c }) => {
					const r = new Map([]);
					try {
						const e = await (
							await import(
								"@codeeditorland/common/Target/Function/Get.js"
							)
						).default(JSON.parse(c.toString()));
						e.get("View") === "Content" &&
							r.set(
								e.get("From"),
								await i(
									e.get("Key"),
									e.get("Identifier"),
									n[e.get("From")],
									"Current",
								),
							),
							t[1].send(
								JSON.stringify({ Original: s(e), Data: s(r) }),
							);
					} catch (e) {
						console.log(e);
					}
				})),
			t[0]
				? new Response(null, { status: 101, webSocket: t[0] })
				: new Response("Can't make a WebSocket.", { status: 404 })
		);
	},
};
const { default: i } = await import(
		"@codeeditorland/common/Target/Function/Access.js"
	),
	{ default: s } = await import(
		"@codeeditorland/common/Target/Function/Put.js"
	),
	{ WebSocketPair: p } = await import(
		"@cloudflare/workers-types/experimental/index.js"
	);
export { i as Access, s as Put, p as WebSocketPair, f as default };
