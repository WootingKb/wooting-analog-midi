module.exports = {
  devServer: {
    open: false,
  },
  webpack: {
    configure: (config) => {
      config.output.publicPath = "";

      return config;
    },
  },
};
