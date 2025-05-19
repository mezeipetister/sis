import { BoardActions } from "./BoardActions";
import DevicesTable from "./DevicesTable";

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
	running_zones: ZoneAction | null;
	zones: ZoneInfo[];
};

// If ZoneAction is not defined elsewhere, you may need to define it as well
// Example placeholder:
type ZoneAction = {
	zone_ids: string[];
	duration_seconds: number;
};

// Szerver komponens, szerver oldali fetch-csel
async function getOnlineClients(): Promise<BoardInfo[]> {
	const res = await fetch("http://server:3400/online_devices", {
		cache: "no-store",
	});
	if (!res.ok) return [];
	const data = await res.json();
	console.log("Fetched data:", data);
	// Ensure the return type matches BoardInfo[]
	return Array.isArray(data) ? data as BoardInfo[] : (data.clients as BoardInfo[]) || [];
}

async function getBoards(): Promise<BoardDetails[]> {
	const res = await fetch("http://server:3400/devices", {
		cache: "no-store",
	});
	if (!res.ok) return [];
	const data = await res.json();
	return Array.isArray(data) ? data : [];
}

export default async function Devices() {
	const onlineClients = await getOnlineClients();
	const boards = await getBoards();
	const newClients = onlineClients.filter((client) => !boards.some((board) => board.device_id === client.device_id));

	return (
		<div>
			<h1 className="pb-5 text-2xl font-bold">Registered devices</h1>
			<DevicesTable clients={onlineClients} boards={boards} />
		</div >
	);
}
