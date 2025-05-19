type BoardInfo = {
	device_id: string;
	datetime: string;
	schedule_version: number;
	running_program: string | null;
	running_zones: {
		zone_ids: string[];
		duration_seconds: number;
	} | null;
};

// Szerver komponens, szerver oldali fetch-csel
async function getOnlineClients() {
	const res = await fetch("http://localhost:3000/api/online-clients", {
		cache: "no-store",
	});
	if (!res.ok) return [];
	const data = await res.json();
	return data.clients || [];
}

export default async function Devices() {
	const clients: BoardInfo[] = await getOnlineClients();

	return (
		<div>
			<h1>Devices</h1>
			{clients.length === 0 ? (
				<p>No online devices.</p>
			) : (
				<table>
					<thead>
						<tr>
							<th>Device ID</th>
							<th>Date/Time</th>
							<th>Schedule Version</th>
							<th>Running Program</th>
							<th>Running Zones</th>
						</tr>
					</thead>
					<tbody>
						{clients.map((client) => (
							<tr key={client.device_id}>
								<td>{client.device_id}</td>
								<td>{client.datetime}</td>
								<td>{client.schedule_version}</td>
								<td>{client.running_program || "-"}</td>
								<td>
									{client.running_zones
										? `${client.running_zones.zone_ids.join(
											", "
										)} (${client.running_zones.duration_seconds}s)`
										: "-"}
								</td>
							</tr>
						))}
					</tbody>
				</table>
			)}
		</div>
	);
}
