import type {NextConfig} from 'next';

const nextConfig: NextConfig = {
  output: 'standalone',
  /* config options here */
  typescript: {
    ignoreBuildErrors: true,
  },
  //   experimental: {
  //     serverActions: {
  //       allowedOrigins: ['localhost'],
  //       // allowedForwardedHosts: ["localhost:3000"],
  //       // ^ You might have to use this property depending on your exact
  //       version.
  //     }
  //   }
};

export default nextConfig;
