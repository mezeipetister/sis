"use client";

import { useState, useTransition } from "react";
import { Program, ZoneAction, sendClientCommand } from "@/app/actions/schedule-actions";
import { ZoneInfo } from "../actions/board-actions";

export default function ClientCommandPanel({
	programs,
	zones,
}: {

	programs: Program[];
	zones: ZoneInfo[];
}) {
	const [isPending, startTransition] = useTransition();
	const [showProgramModal, setShowProgramModal] = useState(false);
	const [showZoneModal, setShowZoneModal] = useState(false);
	const [selectedProgram, setSelectedProgram] = useState("");
	const [selectedZone, setSelectedZone] = useState("");
	const [minutes, setMinutes] = useState(1);

	const handleStop = () => {
		startTransition(async () => {
			await sendClientCommand({ type: "Stop" });
		});
	};

	const handleStartProgram = () => {
		if (!selectedProgram) return;
		sendClientCommand({ type: "StartProgram", program_id: selectedProgram });
		setShowProgramModal(false);
	};

	const handleStartZone = () => {
		if (!selectedZone || minutes < 1) return;
		const zone_action: ZoneAction = {
			zone_ids: [selectedZone],
			duration_seconds: minutes * 60,
		};
		sendClientCommand({ type: "StartZoneAction", zone_action });
		setShowZoneModal(false);
	};

	return (
		<div className="mb-6 space-x-2">
			<button
				className="bg-red-600 text-white px-4 py-2 rounded"
				onClick={handleStop}
			>
				Stop
			</button>
			<button
				className="bg-blue-600 text-white px-4 py-2 rounded"
				onClick={() => setShowProgramModal(true)}
			>
				Start Program
			</button>
			<button
				className="bg-green-600 text-white px-4 py-2 rounded"
				onClick={() => setShowZoneModal(true)}
			>
				Test Zone
			</button>

			{/* StartProgram Modal */}
			{showProgramModal && (
				<div className="fixed inset-0 bg-black bg-opacity-50 flex justify-center items-center z-50">
					<div className="bg-white p-6 rounded shadow space-y-4 w-full max-w-md">
						<h3 className="text-lg font-bold">Start Program</h3>
						<select
							className="w-full border px-3 py-2"
							value={selectedProgram}
							onChange={(e) => setSelectedProgram(e.target.value)}
						>
							<option value="">-- Select Program --</option>
							{programs.map((p) => (
								<option key={p.id} value={p.id}>
									{p.name}
								</option>
							))}
						</select>
						<div className="flex justify-end gap-2">
							<button
								className="text-gray-600"
								onClick={() => setShowProgramModal(false)}
							>
								Mégse
							</button>
							<button
								className="bg-blue-600 text-white px-4 py-2 rounded"
								onClick={handleStartProgram}
								disabled={!selectedProgram}
							>
								Indít
							</button>
						</div>
					</div>
				</div>
			)}

			{/* StartZoneAction Modal */}
			{showZoneModal && (
				<div className="fixed inset-0 bg-black bg-opacity-50 flex justify-center items-center z-50">
					<div className="bg-white p-6 rounded shadow space-y-4 w-full max-w-md">
						<h3 className="text-lg font-bold">Test Zone</h3>
						<select
							className="w-full border px-3 py-2"
							value={selectedZone}
							onChange={(e) => setSelectedZone(e.target.value)}
						>
							<option value="">-- Select Zone --</option>
							{zones.map((z) => (
								<option key={z.id} value={z.id}>
									{z.name || z.id}
								</option>
							))}
						</select>
						<input
							type="number"
							min={1}
							value={minutes}
							onChange={(e) => setMinutes(Number(e.target.value))}
							className="w-full border px-3 py-2"
							placeholder="Minutes"
						/>
						<div className="flex justify-end gap-2">
							<button
								className="text-gray-600"
								onClick={() => setShowZoneModal(false)}
							>
								Mégse
							</button>
							<button
								className="bg-green-600 text-white px-4 py-2 rounded"
								onClick={handleStartZone}
								disabled={!selectedZone || minutes < 1}
							>
								Indít
							</button>
						</div>
					</div>
				</div>
			)}
		</div>
	);
}
