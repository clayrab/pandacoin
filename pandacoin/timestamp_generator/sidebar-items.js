initSidebarItems({"struct":[["SystemTimestampGenerator","This is simply a wrapper for the system clock which allows us to mock the clock. A global reference is kept to once so that we dont’ need to pass a timestamp to every function which requires a timestamp."]],"trait":[["AbstractTimestampGenerator","A trait used for getting timestamps. The purpose of this is to allow the timestamp generator to be mockable. The default implementation simply wraps the system clock."]]});