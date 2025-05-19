import {IncomingMessage} from 'http';
import {MongoClient} from 'mongodb';
import {NextApiRequest, NextApiResponse} from 'next';
import {Duplex} from 'stream';
import {WebSocket, WebSocketServer} from 'ws';

type BoardInfo = {
  device_id: string; datetime: string;
  schedule_version: number, running_program: String | null;
  running_zones: ZoneAction | null
};

type ZoneAction = {
  zone_ids: string[]; duration_seconds: number;
}

type Schedule = {
  version: number; programs: Program[];
}

type Program = {
  id: string; name: string; weekdays: number[]; start_time: string;
  zones: ZoneAction[];
}

export type ServerCommand =|{
  type: 'SetNewSchedule';
  data: Schedule
}
|{type: 'Stop'}|{
  type: 'StartProgram';
  data: string
}
|{
  type: 'StartZoneAction';
  data: ZoneAction
};


// Global variable to store WebSocket connections
const subscribers: Set<WebSocket> = new Set<WebSocket>();

// Map to store the latest BoardInfo for each online client (key: WebSocket,
// value: BoardInfo)
const online_clients: Map<WebSocket, BoardInfo> = new Map();

// Function to broadcast messages to all connected WebSocket clients
const push_schedule = (command: ServerCommand) => {
  const message = JSON.stringify(command);
  for (const client of subscribers) {
    if (client.readyState === WebSocket.OPEN) {
      client.send(message);
    }
  }
};

// WebSocket server setup
const websocketHandler = (req: any, res: any) => {
  if (res.socket.server.wss) {
    res.end();
    return;
  }

  const wss = new WebSocketServer({noServer: true});

  // res.socket.server.on(
  //     'upgrade',
  //     (request: IncomingMessage, socket: Duplex,
  //      head: Buffer<ArrayBufferLike>) => {
  //       console.log('WebSocket server upgrade request');
  //       // Token validation
  //       const token = request.headers['auth_token'] || '';
  //       const expectedToken = process.env.AUTH_TOKEN;
  //       // Accept token as either a single string or array
  //       const tokenStr = Array.isArray(token) ? token[0] : token;
  //       if (!expectedToken || tokenStr !== expectedToken) {
  //         socket.write('HTTP/1.1 401 Unauthorized\r\n\r\n');
  //         socket.destroy();
  //         return;
  //       }
  //       wss.handleUpgrade(request, socket, head, (ws) => {
  //         wss.emit('connection', ws, request);
  //       });
  //     });

  wss.on('connection', (ws) => {
    console.log('New WebSocket connection');
    subscribers.add(ws);

    ws.on('close', () => {
      console.log('WebSocket connection closed');
      subscribers.delete(ws);
      online_clients.delete(ws);
    });

    ws.on('message', (message) => {
      try {
        const parsedMessage: BoardInfo = JSON.parse(message.toString());
        // Store/update the latest BoardInfo for this client
        online_clients.set(ws, parsedMessage);
        console.log('Received message:', parsedMessage);

        const uri = `mongodb://${process.env.MONGODB_URI}`;
        const client = new MongoClient(uri);
        const dbName = 'sis';
        const collectionName = 'board_status';

        const upsertBoardStatus = async (message: BoardInfo) => {
          try {
            await client.connect();
            const db = client.db(dbName);
            const collection = db.collection(collectionName);

            await collection.updateOne(
                {device_id: message.device_id}, {$set: message},
                {upsert: true});

            console.log('Board status upserted successfully');
          } catch (error) {
            console.error('Error upserting board status:', error);
          } finally {
            await client.close();
          }
        };

        upsertBoardStatus(parsedMessage);
      } catch (error) {
        console.error('Invalid message format:', message.toString());
      }
    });
  });

  // Upgrade HTTP request to WebSocket connection
  if (!res.writableEnded) {
    res.writeHead(101, {
      'Content-Type': 'text/plain',
      'Connection': 'Upgrade',
      'Upgrade': 'websocket'
    });
    res.end();
  }

  wss.handleUpgrade(req, req.socket, Buffer.alloc(0), function done(ws) {
    wss.emit('connection', ws, req);
  });
};

export default function handler(req: NextApiRequest, res: NextApiResponse) {
  websocketHandler(req, res);
}

export {websocketHandler, subscribers, push_schedule, online_clients};
