import { getBoards, getOnlineClients } from "../actions/board-actions";
import { BoardActions } from "./BoardActions";
import DevicesTable from "./DevicesTable";



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
