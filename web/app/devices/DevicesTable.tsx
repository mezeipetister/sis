"use client";

import { useState } from "react";
import EditBoardModal from "./EditBoardModal";
import { BoardActions } from "./BoardActions";

type BoardInfo = {
	device_id: string;
	datetime: string;
	schedule_version: number;
	running_program: string | null;
	running_zones: { zone_ids: string[]; duration_seconds: number } | null;
	zones: string[];
};

type ZoneInfo = {
	id: string;
	name: string;
};

type BoardDetails = {
	device_id: string;
	name: string;
	datetime: string;
	schedule_version: number;
	running_program: string | null;
	running_zones: { zone_ids: string[]; duration_seconds: number } | null;
	zones: ZoneInfo[];
};

type Props = {
	clients: BoardInfo[];
	boards: BoardDetails[];
};

export default function DevicesTable({ clients: onlineClients, boards }: Props) {
	const newClients = onlineClients.filter((client) => !boards.some((board) => board.device_id === client.device_id));
	const [editingDevice, setEditingDevice] = useState<BoardDetails | null>(null);

	return (
		<div>
			{boards.length === 0 ? (
				<p>No online devices.</p>
			) : (
				<div className="overflow-x-auto">
					<table className="divide-y divide-gray-200 bg-white shadow rounded-lg border border-gray-300">
						<thead className="bg-gray-50">
							<tr>
								<th className="px-4 py-2 text-xs font-normal text-gray-500 uppercase border-r border-b border-gray-300">Device Info</th>
								<th className="px-6 py-3 text-left text-xs font-normal text-gray-500 uppercase tracking-wider border-b border-gray-300"></th>
							</tr>
						</thead>
						<tbody className="divide-y divide-gray-200">
							{boards.map((client) => (
								<tr key={client.device_id} className="hover:bg-gray-100">
									<td className="px-6 py-4 text-sm text-gray-900 border-r border-gray-300 whitespace-pre-line break-words max-w-xs">
										<strong>Name:</strong> {client.name}
										<br />
										<strong>Device ID:</strong> {client.device_id}
										<br />
										<strong>Status:</strong>{" "}
										<span className={onlineClients.some((c) => c.device_id === client.device_id) ? "text-green-600" : "text-red-600"}>
											{onlineClients.some((c) => c.device_id === client.device_id) ? "Online" : "Offline"}
										</span>
										<br />
										<strong>Schedule version:</strong> {client.schedule_version}
										<br />
										<strong>Updated:</strong> {new Date(client.datetime).toLocaleString("hu-HU", { timeZone: "Europe/Budapest" })}
										<br />
										<strong>Running program:</strong> {client.running_program || "None"}
										<br />
										<strong>Running zones:</strong> {client.running_zones ? client.running_zones.zone_ids.join(", ") : "None"}
										<br />
										<strong>Available zones:</strong>
										<ul className="list-disc list-inside">
											{client.zones.map((zone) => (
												<li key={zone.id}>{zone.name}<br />{zone.id}</li>
											))}
										</ul>
									</td>
									<td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
										<button
											onClick={() => setEditingDevice(client)}
											className="mb-5 px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded hover:bg-blue-700"
										>
											Edit
										</button>
										<br />
										<BoardActions
											device_id={client.device_id}
											add={false}
										/>
									</td>
								</tr>
							))}
						</tbody>
					</table>
				</div>
			)}
			< h1 className="pb-5 text-2xl font-bold">Discovered devices</h1>
			{
				newClients.length === 0 ? (
					<p>No other online devices.</p>
				) : (
					<div className="overflow-x-auto">
						<table className="divide-y divide-gray-200 bg-white shadow rounded-lg border border-gray-300">
							<thead className="bg-gray-50">
								<tr>
									<th className="px-4 py-2 text-xs font-normal text-gray-500 uppercase border-r border-b border-gray-300">Device Info</th>
									<th className="px-6 py-3 text-left text-xs font-normal text-gray-500 uppercase tracking-wider border-b border-gray-300"></th>
								</tr>
							</thead>
							<tbody className="divide-y divide-gray-200">
								{newClients.map((client) => (
									<tr key={client.device_id} className="hover:bg-gray-100">
										<td className="px-6 py-4 text-sm text-gray-900 border-r border-gray-300 whitespace-pre-line break-words max-w-xs">
											Device ID: {client.device_id}
											<br />
											Schedule version: {client.schedule_version}
											<br />
											Updated: {new Date(client.datetime).toLocaleString("hu-HU", { timeZone: "Europe/Budapest" })}
											<br />
											Running program: {client.running_program || "None"}
											<br />
											Running zones: {client.running_zones ? client.running_zones.zone_ids.join(", ") : "None"}
											<br />
											Available zones:
											<ul className="list-disc list-inside">
												{client.zones.map((zone) => (
													<li key={zone}>{zone}</li>
												))}
											</ul>
										</td>
										{!boards.some((board) => board.device_id === client.device_id) && (
											<td className="px-6 py-4 whitespace-nowrap text-sm text-gray-900">
												<BoardActions
													device_id={client.device_id}
													add={true}
												/>
											</td>
										)}
									</tr>
								))}
							</tbody>
						</table>
					</div>
				)
			}

			{/* Modal megjelenítése */}
			{editingDevice && (
				<EditBoardModal
					device_id={editingDevice.device_id}
					initialName={editingDevice.name}
					initialZones={editingDevice.zones}
					onClose={() => setEditingDevice(null)}
				/>
			)}
		</div>
	);
}