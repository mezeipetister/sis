type BoardInfo = {
	device_id: string;
	datetime: string;
	schedule_version: number;
	running_program: string | null;
	running_zones: {
		zone_ids: string[];
		duration_seconds: number;
	} | null;
	zones: string[];
};

// Szerver komponens, szerver oldali fetch-csel
async function getOnlineClients() {
	const res = await fetch("http://192.168.88.30:3400/online_devices", {
		cache: "no-store",
	});
	if (!res.ok) return [];
	const data = await res.json();
	console.log("Fetched data:", data);
	// Adjust this line based on the actual structure of the API response
	return Array.isArray(data) ? data : data.clients || [];
}

export default async function Devices() {
	const clients: BoardInfo[] = await getOnlineClients();

	return (
		<div>
			<h1 className="pb-5 text-2xl font-bold">Devices</h1>
			{clients.length === 0 ? (
				<p>No online devices.</p>
			) : (
				<div className="overflow-x-auto">
					<table className="divide-y divide-gray-200 bg-white shadow rounded-lg border border-gray-300">
						<thead className="bg-gray-50">
							<tr>
								<th className="px-6 py-3 text-left text-xs font-normal text-gray-500 uppercase border-r border-b border-gray-300">Device ID</th>
								<th className="px-4 py-2 text-xs font-normal text-gray-500 uppercase border-r border-b border-gray-300">Schedule Version</th>
								<th className="px-6 py-3 text-left text-xs font-normal text-gray-500 uppercase tracking-wider border-b border-gray-300"></th>
							</tr>
						</thead>
						<tbody className="divide-y divide-gray-200">
							{clients.map((client) => (
								<tr key={client.device_id} className="hover:bg-gray-100">
									<td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 border-r border-gray-300">{client.device_id}</td>
									<td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900 border-r border-gray-300">{client.schedule_version}</td>
									<td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
										<form action={async () => {
											"use server";
											await fetch(`/add_device?id=${encodeURIComponent(client.device_id)}`, {
												method: "POST",
											});
										}}>
											<button
												type="submit"
												className="bg-blue-500 hover:bg-blue-600 text-white font-semibold py-1 px-2 rounded text-xs cursor-pointer"
											>
												Add
											</button>
										</form>
									</td>
								</tr>
							))}
						</tbody>
					</table>
				</div>
			)}
		</div>
	);
}
