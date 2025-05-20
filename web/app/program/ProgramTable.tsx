"use client";

import { enableProgram, disableProgram, removeProgram, Program } from "@/app/actions/schedule-actions";
import { useTransition, useState } from "react";
import EditProgramModal from "./EditProgramModal";
import { ZoneInfo } from "../actions/board-actions";

export default function ProgramTable({ programs, zones }: { programs: Program[], zones: ZoneInfo[] }) {
	const [isPending, startTransition] = useTransition();
	const [editing, setEditing] = useState<any | null>(null);

	return (
		<>
			<table className="min-w-full bg-white shadow rounded-lg border">
				<thead className="bg-gray-100">
					<tr>
						<th className="px-4 py-2 text-left text-sm font-medium">Név</th>
						<th className="px-4 py-2 text-left text-sm font-medium">Kezdés</th>
						<th className="px-4 py-2 text-left text-sm font-medium">Napok</th>
						<th className="px-4 py-2 text-left text-sm font-medium">Zónák</th>
						<th className="px-4 py-2 text-left text-sm font-medium">Műveletek</th>
					</tr>
				</thead>
				<tbody>
					{programs.map((p) => (
						<tr key={p.id} className="border-t">
							<td className="px-4 py-2">{p.name}</td>
							<td className="px-4 py-2">{p.start_time}</td>
							<td className="px-4 py-2">
								{p.weekdays
									.map((d: number) =>
										["Hétfő", "Kedd", "Szerda", "Csütörtök", "Péntek", "Szombat", "Vasárnap"][d - 1]
									)
									.join(", ")}
							</td>
							<td className="px-4 py-2">
								{p.zones.map((z: any, i: number) => {
									const zoneNames = z.zone_ids
										.map((id: number) => zones.find((zone) => String(zone.id) === String(id))?.name)
										.filter(Boolean)
										.join(", ");
									return (
										<div key={i}>
											{zoneNames} ({Math.round(z.duration_seconds / 60)}m)
										</div>
									);
								})}
							</td>
							<td className="px-4 py-2 space-x-2">
								<button
									className="px-3 py-1 bg-yellow-500 text-white rounded"
									onClick={() => setEditing(p)}
								>
									Szerkesztés
								</button>
								<button
									className={`px-3 py-1 text-white rounded ${p.active ? "bg-gray-500" : "bg-green-600"}`}
									onClick={() =>
										startTransition(async () => {
											if (p.active) {
												await disableProgram(p.id);
											} else {
												await enableProgram(p.id);
											}
											window.location.reload();
										})
									}
								>
									{p.active ? "Kikapcsolás" : "Engedélyezés"}
								</button>
								{!p.active && (
									<button
										className="px-3 py-1 bg-red-600 text-white rounded"
										onClick={() => {
											if (window.confirm("Biztosan törölni szeretnéd ezt a programot?")) {
												startTransition(async () => {
													await removeProgram(p.id);
													window.location.reload();
												});
											}
										}}
									>
										Törlés
									</button>
								)}
							</td>
						</tr>
					))}
				</tbody>
			</table>

			{editing && (
				<EditProgramModal program={editing} zones={zones} onClose={() => setEditing(null)} />
			)}
		</>
	);
}