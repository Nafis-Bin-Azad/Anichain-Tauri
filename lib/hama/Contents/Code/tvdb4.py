### tvdb4 ###

### Imports ###               ### Functions used ###
# Python Modules #
import os
# HAMA Modules #
import common
from common import Log, DictString, Dict, SaveDict # Direct import of heavily used functions

### Variables ###
TVDB4_MAPPING_URL = 'https://raw.githubusercontent.com/ZeroQI/Absolute-Series-Scanner/master/tvdb4.mapping.xml'
TVDB4_POSTERS_URL = 'https://raw.githubusercontent.com/ZeroQI/Absolute-Series-Scanner/master/tvdb4.posters.xml'

### Functions ###
def GetMetadata(media, movie, source, TVDBid, mappingList, num=0):
  """ [tvdb4.mapping.xml] Attempt to get the ASS's episode mapping data
      [tvdb4.posters.xml] Attempt to get the ASS's image data
  """
  Log.Info('=== tvdb4.GetMetadata() ==='.ljust(157, '='))
  TVDB4_dict, TVDB4_mapping, TVDB4_posters = {}, None, None

  if movie or not source == "tvdb4":  Log.Info("not tvdb4 mode");  return TVDB4_dict
  Log.Info("tvdb4 mode")

  def find_tvdb4_file(file_to_find):
    try:
      folder = common.GetMediaDir(media, movie)
      while folder and folder[-1] not in ["/", "\\"]:
        filename = os.path.join(folder, file_to_find)
        if os.path.exists(filename):
          file = Core.storage.load(os.path.realpath(filename))
          try:     return XML.ElementFromString(file)
          except:  return file
        folder = os.path.dirname(folder)
      else: Log.Info("No '{}' file detected locally".format(file_to_find))
    except Exception as e:  Log.Error("Issues in finding setup info as directories have most likely changed post scan into Plex, Exception: '%s'" % e)
    return ""

  Log.Info("--- tvdb4.mapping.xml ---".ljust(157, '-'))
  TVDB4_mapping = find_tvdb4_file("tvdb4.mapping")
  
  if TVDB4_mapping: Log.Debug("'tvdb4.mapping' file detected locally")
  else:             TVDB4_mapping = TVDB4_mapping or common.LoadFile(filename=os.path.basename(TVDB4_MAPPING_URL), url=TVDB4_MAPPING_URL)
  entry = ""
  if isinstance(TVDB4_mapping, str):  entry = TVDB4_mapping
  else:
    entry = common.GetXml(TVDB4_mapping, "/tvdb4entries/anime[@tvdbid='%s']" % TVDBid)
    if not entry:  Log.Error("TVDBid '%s' is not found in mapping file" % TVDBid)
  if entry:
    for line in filter(None, entry.strip().splitlines()):
      season = line.strip().split("|")
      for absolute_episode in range(int(season[1]), int(season[2])+1):  SaveDict((str(int(season[0])), str(absolute_episode)), mappingList, 'absolute_map', str(absolute_episode))
      SaveDict(True if "(unknown length)" in season[3] or season[1] == season[2] else False, mappingList, 'absolute_map', 'unknown_series_length')
      SaveDict(str(int(season[0])), mappingList, 'absolute_map', 'max_season')
      Log.Info("[ ] season: {}, starting episode: {}, ending episode: {}, label: {}".format(season[0], season[1], season[2], season[3]))

  Log.Info("--- tvdb4.posters.xml ---".ljust(157, '-'))
  TVDB4_posters = find_tvdb4_file("tvdb.posters")
  
  if TVDB4_posters: Log.Debug("'tvdb4.posters' file detected locally")
  else:         TVDB4_posters  = TVDB4_posters or common.LoadFile(filename=os.path.basename(TVDB4_POSTERS_URL), url=TVDB4_POSTERS_URL)
  entry = ""
  if isinstance(TVDB4_posters, str):  entry = TVDB4_posters
  else:
    entry = common.GetXml(TVDB4_posters, "/tvdb4entries/posters[@tvdbid='%s']" % TVDBid)
    if not entry:  Log.Error("TVDBid '%s' is not found in posters file" % TVDBid)
  if TVDB4_posters and entry:
    season_posters = {}
    for line in filter(None, entry.strip().splitlines()):
      season, url       = line.strip().split("|",1)
      season            = season.lstrip("0") if season.lstrip("0") else "0"
      SaveDict(Dict(season_posters, season, default=0)+1, season_posters, season)
      rank = common.poster_rank('tvdb4', 'posters', rank_adjustment=season_posters[season]-1)
      SaveDict(("TheTVDB/seasons/%s-%s-%s" % (TVDBid, season, os.path.basename(url)), rank, None), TVDB4_dict, 'seasons', season, 'posters', url)
      Log.Info("[ ] season: {:>2}, rank: {:>3}, filename: {}".format(season, rank, url))

  Log.Info("--- return ---".ljust(157, '-'))
  Log.Info("absolute_map: {}".format(DictString(Dict(mappingList, 'absolute_map', default={}), 0)))
  Log.Info("TVDB4_dict: {}".format(DictString(TVDB4_dict, 4)))
  return TVDB4_dict
