<script>
	import "../app.css";

	// Handle Qt Initialisation
	import { ensureQtReady } from "$lib/qt/qt.ts";
	import { onMount } from "svelte";

	onMount(async () => {
			await ensureQtReady();
		}
	);

</script>

<slot />
